#![no_std]

//! A smart account that can be pulled out of a position by a keeper, and by no one else,
//! and only in the exact way its owner wrote down.
//!
//! The account has two ways to authorize a call. An owner signature carries full authority,
//! the way any wallet does. A rule id carries none of its own: it authorizes one function on
//! one contract, and only while the guardian confirms that rule's trigger holds right now.
//!
//! A keeper presenting a rule id is making a claim, not exercising a permission. The guardian
//! is what turns the claim into authority, and it re-derives the trigger from the oracle
//! before it does. Disarm the rule and the claim buys nothing, whatever the guardian says.

mod error;

#[cfg(test)]
mod test;

pub use error::Error;

use soroban_sdk::auth::{Context, ContractContext, CustomAccountInterface};
use soroban_sdk::crypto::Hash;
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, Address, Bytes, BytesN, Env, Symbol, Vec,
};

/// Ledgers in a day at Stellar's ~5s close time.
const DAY: u32 = 17_280;
const ARMED_TTL: u32 = 30 * DAY;
const ARMED_BUMP: u32 = 90 * DAY;

#[contractclient(name = "GuardianClient")]
pub trait GuardianInterface {
    fn check(env: Env, id: u32) -> bool;
}

#[contracttype]
pub enum DataKey {
    Owner,
    Guardian,
    Armed(u32),
}

/// The entire authority a rule has over this account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Armed {
    pub position: Address,
    pub fn_name: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Signature {
    /// The account's own key.
    Owner(BytesN<64>),
    /// A keeper's claim that rule `id` currently holds.
    Rule(u32),
}

#[contract]
pub struct GuardedAccount;

#[contractimpl]
impl GuardedAccount {
    pub fn __constructor(env: Env, owner: BytesN<32>, guardian: Address) {
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Guardian, &guardian);
    }

    /// Grant rule `id` the right to call `fn_name` on `position`, and nothing else.
    pub fn arm(env: Env, id: u32, position: Address, fn_name: Symbol) {
        env.current_contract_address().require_auth();

        let key = DataKey::Armed(id);
        env.storage()
            .persistent()
            .set(&key, &Armed { position, fn_name });
        env.storage()
            .persistent()
            .extend_ttl(&key, ARMED_TTL, ARMED_BUMP);
    }

    /// Revoke rule `id`. Takes effect immediately and needs no one's cooperation.
    pub fn disarm(env: Env, id: u32) {
        env.current_contract_address().require_auth();
        env.storage().persistent().remove(&DataKey::Armed(id));
    }

    pub fn armed(env: Env, id: u32) -> Option<Armed> {
        env.storage().persistent().get(&DataKey::Armed(id))
    }

    pub fn guardian(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Guardian)
    }
}

#[contractimpl]
impl CustomAccountInterface for GuardedAccount {
    type Signature = Signature;
    type Error = Error;

    fn __check_auth(
        env: Env,
        payload: Hash<32>,
        signature: Signature,
        contexts: Vec<Context>,
    ) -> Result<(), Error> {
        match signature {
            Signature::Owner(sig) => check_owner(&env, &payload, &sig),
            Signature::Rule(id) => check_rule(&env, id, &contexts),
        }
    }
}

fn check_owner(env: &Env, payload: &Hash<32>, signature: &BytesN<64>) -> Result<(), Error> {
    let owner: BytesN<32> = env
        .storage()
        .instance()
        .get(&DataKey::Owner)
        .ok_or(Error::NotInitialized)?;

    let message: Bytes = payload.to_bytes().into();
    env.crypto().ed25519_verify(&owner, &message, signature);
    Ok(())
}

fn check_rule(env: &Env, id: u32, contexts: &Vec<Context>) -> Result<(), Error> {
    let armed: Armed = env
        .storage()
        .persistent()
        .get(&DataKey::Armed(id))
        .ok_or(Error::RuleNotArmed)?;

    if contexts.is_empty() {
        return Err(Error::NoContexts);
    }

    for context in contexts.iter() {
        let ContractContext {
            contract, fn_name, ..
        } = match context {
            Context::Contract(c) => c,
            // Deploying contracts is not something a stop-loss needs to do.
            _ => return Err(Error::ContractNotAllowed),
        };

        // No rule may ever reach this account's own configuration. Without this, a rule
        // armed against `arm` or `disarm` would let the keeper path rewrite the limits it
        // is supposed to be bound by.
        if contract == env.current_contract_address() {
            return Err(Error::ContractNotAllowed);
        }
        if contract != armed.position {
            return Err(Error::ContractNotAllowed);
        }
        if fn_name != armed.fn_name {
            return Err(Error::FunctionNotAllowed);
        }
    }

    let guardian: Address = env
        .storage()
        .instance()
        .get(&DataKey::Guardian)
        .ok_or(Error::NotInitialized)?;

    if !GuardianClient::new(env, &guardian).check(&id) {
        return Err(Error::RuleNotTriggered);
    }

    Ok(())
}
