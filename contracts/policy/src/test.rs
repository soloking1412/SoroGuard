#![cfg(test)]

extern crate std;

use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::auth::{Context, ContractContext};
use soroban_sdk::testutils::{Address as _, BytesN as _};
use soroban_sdk::{
    contract, contractimpl, symbol_short, vec, Address, BytesN, Env, IntoVal, InvokeError, Symbol,
    Vec,
};

use crate::{Error, GuardedAccount, GuardedAccountClient, Signature};

const RULE: u32 = 7;
const PAYLOAD: [u8; 32] = [9u8; 32];

/// Stands in for the guardian. The real one re-derives the trigger from a SEP-40 feed; what
/// matters here is only that the policy refuses to move without a yes.
#[contract]
pub struct MockGuardian;

#[contractimpl]
impl MockGuardian {
    pub fn set(env: Env, holds: bool) {
        env.storage().instance().set(&symbol_short!("h"), &holds);
    }

    pub fn check(env: Env, _id: u32) -> bool {
        env.storage()
            .instance()
            .get(&symbol_short!("h"))
            .unwrap_or(false)
    }
}

struct Harness {
    env: Env,
    account: Address,
    client: GuardedAccountClient<'static>,
    guardian: MockGuardianClient<'static>,
    position: Address,
    key: SigningKey,
}

fn setup() -> Harness {
    let env = Env::default();
    env.mock_all_auths();

    let guardian_id = env.register(MockGuardian, ());
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let owner = BytesN::from_array(&env, &key.verifying_key().to_bytes());
    let account = env.register(GuardedAccount, (owner, guardian_id.clone()));

    Harness {
        client: GuardedAccountClient::new(&env, &account),
        guardian: MockGuardianClient::new(&env, &guardian_id),
        position: Address::generate(&env),
        account,
        key,
        env,
    }
}

impl Harness {
    fn contexts(&self, calls: &[(&Address, Symbol)]) -> Vec<Context> {
        let mut out = vec![&self.env];
        for (contract, fn_name) in calls {
            out.push_back(Context::Contract(ContractContext {
                contract: (*contract).clone(),
                fn_name: fn_name.clone(),
                args: vec![&self.env],
            }));
        }
        out
    }

    /// Emulates the host calling `__check_auth` during a `require_auth`.
    fn check_auth(
        &self,
        signature: Signature,
        contexts: Vec<Context>,
    ) -> Result<(), Result<Error, InvokeError>> {
        self.env.try_invoke_contract_check_auth::<Error>(
            &self.account,
            &BytesN::random(&self.env),
            signature.into_val(&self.env),
            &contexts,
        )
    }

    fn arm_exit(&self) {
        self.client
            .arm(&RULE, &self.position, &symbol_short!("exit_to"));
    }

    /// The owner path, over a fixed payload the test can sign for real.
    fn check_signed(
        &self,
        key: &SigningKey,
        contexts: Vec<Context>,
    ) -> Result<(), Result<Error, InvokeError>> {
        let signature = Signature::Owner(BytesN::from_array(
            &self.env,
            &key.sign(&PAYLOAD).to_bytes(),
        ));

        self.env.try_invoke_contract_check_auth::<Error>(
            &self.account,
            &BytesN::from_array(&self.env, &PAYLOAD),
            signature.into_val(&self.env),
            &contexts,
        )
    }
}

/// The keys never leave the user. A rule is a second, narrower way in, not a replacement for
/// the first.
#[test]
fn the_owner_key_still_authorizes_anything() {
    let h = setup();
    let unrelated = Address::generate(&h.env);
    let contexts = h.contexts(&[(&unrelated, symbol_short!("transfer"))]);

    assert_eq!(h.check_signed(&h.key, contexts), Ok(()));
}

#[test]
fn a_signature_from_the_wrong_key_authorizes_nothing() {
    let h = setup();
    let attacker = SigningKey::from_bytes(&[8u8; 32]);
    let contexts = h.contexts(&[(&h.position, symbol_short!("exit_to"))]);

    assert_eq!(
        h.check_signed(&attacker, contexts),
        Err(Err(InvokeError::Abort))
    );
}

#[test]
fn arming_records_exactly_one_function_on_one_contract() {
    let h = setup();
    h.arm_exit();

    let armed = h.client.armed(&RULE).unwrap();
    assert_eq!(armed.position, h.position);
    assert_eq!(armed.fn_name, symbol_short!("exit_to"));
}

#[test]
fn a_keeper_may_exit_the_position_when_the_guardian_agrees() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    let contexts = h.contexts(&[(&h.position, symbol_short!("exit_to"))]);
    assert_eq!(h.check_auth(Signature::Rule(RULE), contexts), Ok(()));
}

#[test]
fn a_keeper_may_do_nothing_when_the_guardian_disagrees() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&false);

    let contexts = h.contexts(&[(&h.position, symbol_short!("exit_to"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::RuleNotTriggered))
    );
}

#[test]
fn a_rule_that_was_never_armed_carries_no_authority() {
    let h = setup();
    h.guardian.set(&true);

    let contexts = h.contexts(&[(&h.position, symbol_short!("exit_to"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(404), contexts),
        Err(Ok(Error::RuleNotArmed))
    );
}

/// The revocation claim: the owner can stop this at any time, without asking anyone, and it
/// takes effect even mid-trigger.
#[test]
fn disarming_stops_the_keeper_while_the_trigger_still_holds() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    let contexts = h.contexts(&[(&h.position, symbol_short!("exit_to"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts.clone()),
        Ok(())
    );

    h.client.disarm(&RULE);

    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::RuleNotArmed))
    );
}

#[test]
fn a_rule_cannot_reach_a_contract_it_was_not_armed_against() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    let elsewhere = Address::generate(&h.env);
    let contexts = h.contexts(&[(&elsewhere, symbol_short!("exit_to"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::ContractNotAllowed))
    );
}

#[test]
fn a_rule_cannot_call_a_function_it_was_not_armed_against() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    let contexts = h.contexts(&[(&h.position, symbol_short!("transfer"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::FunctionNotAllowed))
    );
}

/// Without the self-reference guard, a rule armed against this account's own `arm` would let
/// the keeper path rewrite the limits binding it.
#[test]
fn a_rule_cannot_rearm_the_account_that_granted_it() {
    let h = setup();
    h.guardian.set(&true);
    h.client.arm(&RULE, &h.account, &symbol_short!("arm"));

    let contexts = h.contexts(&[(&h.account, symbol_short!("arm"))]);
    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::ContractNotAllowed))
    );
}

#[test]
fn a_rule_authorizes_nothing_on_an_empty_context() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    assert_eq!(
        h.check_auth(Signature::Rule(RULE), vec![&h.env]),
        Err(Ok(Error::NoContexts))
    );
}

/// One allowed call does not carry an unrelated one alongside it.
#[test]
fn every_call_in_a_batch_must_match_the_armed_rule() {
    let h = setup();
    h.arm_exit();
    h.guardian.set(&true);

    let elsewhere = Address::generate(&h.env);
    let contexts = h.contexts(&[
        (&h.position, symbol_short!("exit_to")),
        (&elsewhere, symbol_short!("exit_to")),
    ]);

    assert_eq!(
        h.check_auth(Signature::Rule(RULE), contexts),
        Err(Ok(Error::ContractNotAllowed))
    );
}
