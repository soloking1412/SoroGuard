use soroban_sdk::{contracttype, Env};

use crate::error::Error;
use crate::rule::Rule;

/// Ledgers in a day at Stellar's ~5s close time.
const DAY: u32 = 17_280;

const RULE_TTL: u32 = 30 * DAY;
const RULE_BUMP: u32 = 90 * DAY;
const INSTANCE_TTL: u32 = 30 * DAY;
const INSTANCE_BUMP: u32 = 90 * DAY;

#[contracttype]
pub enum DataKey {
    NextId,
    Rule(u32),
}

pub fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL, INSTANCE_BUMP);
}

pub fn next_id(env: &Env) -> u32 {
    let id: u32 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);
    env.storage().instance().set(&DataKey::NextId, &(id + 1));
    id
}

pub fn save(env: &Env, id: u32, rule: &Rule) {
    let key = DataKey::Rule(id);
    env.storage().persistent().set(&key, rule);
    env.storage()
        .persistent()
        .extend_ttl(&key, RULE_TTL, RULE_BUMP);
}

pub fn load(env: &Env, id: u32) -> Result<Rule, Error> {
    let key = DataKey::Rule(id);
    let rule: Rule = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::RuleNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, RULE_TTL, RULE_BUMP);
    Ok(rule)
}

/// Read without touching the entry's TTL, for read-only paths.
pub fn peek(env: &Env, id: u32) -> Result<Rule, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Rule(id))
        .ok_or(Error::RuleNotFound)
}
