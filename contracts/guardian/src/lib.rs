#![no_std]

//! The guardian stores a user's protection rules and decides, from on-chain state alone,
//! whether one of them currently holds.
//!
//! It holds no funds and it takes no price, health factor or proof from its caller. A keeper
//! can only ask it to look. If the rule doesn't hold when the guardian looks, nothing runs.

mod adapter;
mod error;
mod event;
mod oracle;
mod rule;
mod storage;
mod trigger;

#[cfg(test)]
mod test;

pub use adapter::{HealthAdapter, HealthClient, PositionAdapter, PositionClient};
pub use error::Error;
pub use oracle::{Asset, PriceData, PriceFeedClient, PriceFeedTrait};
pub use rule::*;

use soroban_sdk::{contract, contractimpl, Address, Env};

use event::{Cancelled, Fired, Registered};

/// Fixed-point scale for health factors. 1.0 is `10_000_000`.
pub const HEALTH_DECIMALS: u32 = 7;

/// Shortest cooldown a rule may carry, in seconds.
///
/// A rule that can fire every ledger is a rule a keeper can act on during a wick.
pub const MIN_COOLDOWN: u64 = 60;

#[contract]
pub struct Guardian;

#[contractimpl]
impl Guardian {
    /// Store a rule for `owner` and return its id.
    ///
    /// Takes the rule's parts rather than a `Rule`, so a caller has no field through which
    /// to pass their own `last_fired` or `status`.
    pub fn register(
        env: Env,
        owner: Address,
        position: Address,
        trigger: Trigger,
        action: Action,
        cooldown: u64,
    ) -> Result<u32, Error> {
        owner.require_auth();
        validate(&trigger, cooldown)?;

        let rule = Rule {
            owner,
            position,
            trigger,
            action,
            cooldown,
            last_fired: 0,
            status: RuleStatus::Active,
        };

        let id = storage::next_id(&env);
        storage::save(&env, id, &rule);
        storage::extend_instance(&env);

        Registered {
            id,
            owner: rule.owner,
        }
        .publish(&env);
        Ok(id)
    }

    /// Retire a rule. Only the owner can, and it can't be undone.
    pub fn cancel(env: Env, id: u32) -> Result<(), Error> {
        let mut rule = storage::load(&env, id)?;
        rule.owner.require_auth();

        if rule.status != RuleStatus::Active {
            return Err(Error::RuleInactive);
        }

        rule.status = RuleStatus::Cancelled;
        storage::save(&env, id, &rule);

        Cancelled {
            id,
            owner: rule.owner,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_rule(env: Env, id: u32) -> Result<Rule, Error> {
        storage::peek(&env, id)
    }

    /// Whether the rule would fire right now.
    ///
    /// Read-only, and gated identically to [`Guardian::execute`], so a keeper simulating this
    /// and a policy consulting it during `__check_auth` reach the same answer as the
    /// execution itself.
    pub fn check(env: Env, id: u32) -> Result<bool, Error> {
        let rule = storage::peek(&env, id)?;

        if rule.status != RuleStatus::Active || in_cooldown(&env, &rule) {
            return Ok(false);
        }

        trigger::holds(&env, &rule.owner, &rule.trigger)
    }

    /// Re-derive the trigger and, if it holds, run the rule's action.
    ///
    /// Returns the amount the action moved.
    pub fn execute(env: Env, id: u32, keeper: Address) -> Result<i128, Error> {
        keeper.require_auth();

        let mut rule = storage::load(&env, id)?;

        if rule.status != RuleStatus::Active {
            return Err(Error::RuleInactive);
        }
        if in_cooldown(&env, &rule) {
            return Err(Error::CooldownActive);
        }
        if !trigger::holds(&env, &rule.owner, &rule.trigger)? {
            return Err(Error::TriggerNotMet);
        }

        rule.last_fired = env.ledger().timestamp();
        storage::save(&env, id, &rule);

        let position = PositionClient::new(&env, &rule.position);
        let moved = match &rule.action {
            Action::ExitToStable(a) => position.exit_to(&rule.owner, &a.to),
            Action::Repay(a) => position.repay(&rule.owner, &a.asset),
        };

        Fired { id, keeper, moved }.publish(&env);
        Ok(moved)
    }
}

fn in_cooldown(env: &Env, rule: &Rule) -> bool {
    rule.last_fired != 0 && env.ledger().timestamp() < rule.last_fired + rule.cooldown
}

fn validate(trigger: &Trigger, cooldown: u64) -> Result<(), Error> {
    if cooldown < MIN_COOLDOWN {
        return Err(Error::CooldownTooShort);
    }

    match trigger {
        Trigger::Price(t) if t.below <= 0 => Err(Error::NonPositiveThreshold),
        Trigger::Health(t) if t.floor <= 0 => Err(Error::NonPositiveThreshold),
        Trigger::Deviation(t) if t.feeds.len() < 2 => Err(Error::NotEnoughFeeds),
        Trigger::Deviation(t) if t.max_bps == 0 => Err(Error::NonPositiveThreshold),
        _ => Ok(()),
    }
}
