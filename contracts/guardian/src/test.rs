#![cfg(test)]

extern crate std;

use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env, Vec};

use crate::adapter::{HealthAdapter, PositionAdapter};
use crate::oracle::{Asset, PriceData, PriceFeedTrait};
use crate::{
    Action, Error, ExitToStable, FeedDeviation, Guardian, GuardianClient, HealthFloor, PriceBelow,
    Trigger,
};

const NOW: u64 = 1_770_000_000;
const COOLDOWN: u64 = 300;
const MAX_AGE: u64 = 900;

/// A SEP-40 feed whose price, timestamp and decimals the test sets directly.
#[contract]
pub struct MockFeed;

#[contractimpl]
impl MockFeed {
    pub fn set(env: Env, price: i128, timestamp: u64, decimals: u32) {
        env.storage().instance().set(&symbol_short!("p"), &price);
        env.storage()
            .instance()
            .set(&symbol_short!("t"), &timestamp);
        env.storage().instance().set(&symbol_short!("d"), &decimals);
    }

    pub fn go_dark(env: Env) {
        env.storage().instance().remove(&symbol_short!("p"));
    }
}

#[contractimpl]
impl PriceFeedTrait for MockFeed {
    fn base(_env: Env) -> Asset {
        Asset::Other(symbol_short!("USD"))
    }

    fn assets(env: Env) -> Vec<Asset> {
        vec![&env]
    }

    fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("d"))
            .unwrap_or(14)
    }

    fn resolution(_env: Env) -> u32 {
        300
    }

    fn price(_env: Env, _asset: Asset, _timestamp: u64) -> Option<PriceData> {
        None
    }

    fn prices(_env: Env, _asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        None
    }

    fn lastprice(env: Env, _asset: Asset) -> Option<PriceData> {
        Some(PriceData {
            price: env.storage().instance().get(&symbol_short!("p"))?,
            timestamp: env
                .storage()
                .instance()
                .get(&symbol_short!("t"))
                .unwrap_or(0),
        })
    }
}

/// A position that records how many times it was unwound.
#[contract]
pub struct MockPosition;

#[contractimpl]
impl MockPosition {
    pub fn exits(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("n"))
            .unwrap_or(0)
    }
}

#[contractimpl]
impl PositionAdapter for MockPosition {
    fn exit_to(env: Env, owner: Address, _to: Address) -> i128 {
        owner.require_auth();
        let n: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("n"))
            .unwrap_or(0);
        env.storage().instance().set(&symbol_short!("n"), &(n + 1));
        1_000
    }

    fn repay(env: Env, owner: Address, _asset: Address) -> i128 {
        owner.require_auth();
        let n: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("n"))
            .unwrap_or(0);
        env.storage().instance().set(&symbol_short!("n"), &(n + 1));
        500
    }
}

#[contract]
pub struct MockHealth;

#[contractimpl]
impl MockHealth {
    pub fn set(env: Env, health: i128) {
        env.storage().instance().set(&symbol_short!("h"), &health);
    }
}

#[contractimpl]
impl HealthAdapter for MockHealth {
    fn health_factor(env: Env, _owner: Address) -> i128 {
        env.storage().instance().get(&symbol_short!("h")).unwrap()
    }
}

struct Harness {
    env: Env,
    guardian: GuardianClient<'static>,
    owner: Address,
    keeper: Address,
    position: Address,
    usdc: Address,
    feed: MockFeedClient<'static>,
    feed_id: Address,
}

fn setup() -> Harness {
    let env = Env::default();
    // The keeper signs the root call. The owner's authorization for the sub-invocation into
    // their position comes from their smart account's policy, not from the keeper's
    // transaction, so it is by construction not rooted in it.
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().with_mut(|l| l.timestamp = NOW);

    let guardian_id = env.register(Guardian, ());
    let position = env.register(MockPosition, ());
    let feed_id = env.register(MockFeed, ());

    let feed = MockFeedClient::new(&env, &feed_id);
    feed.set(&110_000_000_000_000, &NOW, &14);

    Harness {
        guardian: GuardianClient::new(&env, &guardian_id),
        owner: Address::generate(&env),
        keeper: Address::generate(&env),
        usdc: Address::generate(&env),
        position,
        feed,
        feed_id,
        env,
    }
}

impl Harness {
    /// A stop-loss that fires under $0.11, at the feed's 14 decimals.
    fn stop_loss(&self) -> Trigger {
        Trigger::Price(PriceBelow {
            asset: Asset::Other(symbol_short!("XLM")),
            below: 11_000_000_000_000,
            oracle: self.feed_id.clone(),
            max_age: MAX_AGE,
        })
    }

    fn exit_to_usdc(&self) -> Action {
        Action::ExitToStable(ExitToStable {
            to: self.usdc.clone(),
        })
    }

    fn register(&self, trigger: Trigger) -> u32 {
        self.guardian.register(
            &self.owner,
            &self.position,
            &trigger,
            &self.exit_to_usdc(),
            &COOLDOWN,
        )
    }

    fn exits(&self) -> u32 {
        MockPositionClient::new(&self.env, &self.position).exits()
    }

    fn set_price(&self, price: i128) {
        self.feed.set(&price, &NOW, &14);
    }
}

#[test]
fn registers_a_rule_and_reads_it_back() {
    let h = setup();
    let id = h.register(h.stop_loss());

    let rule = h.guardian.get_rule(&id);
    assert_eq!(rule.owner, h.owner);
    assert_eq!(rule.cooldown, COOLDOWN);
    assert_eq!(rule.last_fired, 0);
}

#[test]
fn rejects_a_cooldown_short_enough_to_act_on_a_wick() {
    let h = setup();
    let result = h.guardian.try_register(
        &h.owner,
        &h.position,
        &h.stop_loss(),
        &h.exit_to_usdc(),
        &30,
    );
    assert_eq!(result, Err(Ok(Error::CooldownTooShort)));
}

#[test]
fn rejects_a_deviation_rule_with_only_one_feed() {
    let h = setup();
    let trigger = Trigger::Deviation(FeedDeviation {
        asset: Asset::Other(symbol_short!("USTRY")),
        feeds: vec![&h.env, h.feed_id.clone()],
        max_bps: 500,
        max_age: MAX_AGE,
    });

    let result = h.guardian.try_register(
        &h.owner,
        &h.position,
        &trigger,
        &h.exit_to_usdc(),
        &COOLDOWN,
    );
    assert_eq!(result, Err(Ok(Error::NotEnoughFeeds)));
}

#[test]
fn does_nothing_while_the_price_holds_above_the_floor() {
    let h = setup();
    let id = h.register(h.stop_loss());

    assert!(!h.guardian.check(&id));
    assert_eq!(
        h.guardian.try_execute(&id, &h.keeper),
        Err(Ok(Error::TriggerNotMet))
    );
    assert_eq!(h.exits(), 0);
}

#[test]
fn exits_the_position_once_the_price_breaks_the_floor() {
    let h = setup();
    let id = h.register(h.stop_loss());

    h.set_price(10_500_000_000_000);

    assert!(h.guardian.check(&id));
    assert_eq!(h.guardian.execute(&id, &h.keeper), 1_000);
    assert_eq!(h.exits(), 1);
    assert_eq!(h.guardian.get_rule(&id).last_fired, NOW);
}

#[test]
fn will_not_act_on_a_stale_feed() {
    let h = setup();
    let id = h.register(h.stop_loss());

    h.feed.set(&10_500_000_000_000, &(NOW - MAX_AGE - 1), &14);

    assert_eq!(h.guardian.try_check(&id), Err(Ok(Error::OracleStale)));
    assert_eq!(
        h.guardian.try_execute(&id, &h.keeper),
        Err(Ok(Error::OracleStale))
    );
    assert_eq!(h.exits(), 0);
}

#[test]
fn will_not_act_when_the_feed_has_no_price() {
    let h = setup();
    let id = h.register(h.stop_loss());

    h.feed.go_dark();

    assert_eq!(
        h.guardian.try_execute(&id, &h.keeper),
        Err(Ok(Error::OracleUnavailable))
    );
}

#[test]
fn holds_the_cooldown_before_firing_again() {
    let h = setup();
    let id = h.register(h.stop_loss());
    h.set_price(10_500_000_000_000);

    h.guardian.execute(&id, &h.keeper);

    h.env
        .ledger()
        .with_mut(|l| l.timestamp = NOW + COOLDOWN - 1);
    assert!(!h.guardian.check(&id));
    assert_eq!(
        h.guardian.try_execute(&id, &h.keeper),
        Err(Ok(Error::CooldownActive))
    );
    assert_eq!(h.exits(), 1);

    h.env.ledger().with_mut(|l| l.timestamp = NOW + COOLDOWN);
    h.feed.set(&10_500_000_000_000, &(NOW + COOLDOWN), &14);
    assert_eq!(h.guardian.execute(&id, &h.keeper), 1_000);
    assert_eq!(h.exits(), 2);
}

#[test]
fn a_cancelled_rule_stops_firing() {
    let h = setup();
    let id = h.register(h.stop_loss());
    h.set_price(10_500_000_000_000);

    h.guardian.cancel(&id);

    assert!(!h.guardian.check(&id));
    assert_eq!(
        h.guardian.try_execute(&id, &h.keeper),
        Err(Ok(Error::RuleInactive))
    );
    assert_eq!(h.exits(), 0);
}

#[test]
fn cancelling_twice_is_rejected() {
    let h = setup();
    let id = h.register(h.stop_loss());

    h.guardian.cancel(&id);
    assert_eq!(h.guardian.try_cancel(&id), Err(Ok(Error::RuleInactive)));
}

#[test]
fn an_unknown_rule_id_is_not_a_rule() {
    let h = setup();
    assert_eq!(h.guardian.try_check(&404), Err(Ok(Error::RuleNotFound)));
}

#[test]
fn exits_a_loan_when_health_reaches_the_floor() {
    let h = setup();
    let adapter = h.env.register(MockHealth, ());
    MockHealthClient::new(&h.env, &adapter).set(&12_500_000);

    let id = h.register(Trigger::Health(HealthFloor {
        adapter: adapter.clone(),
        floor: 11_000_000,
    }));

    assert!(!h.guardian.check(&id));

    MockHealthClient::new(&h.env, &adapter).set(&10_900_000);
    assert!(h.guardian.check(&id));
    assert_eq!(h.guardian.execute(&id, &h.keeper), 1_000);
}

#[test]
fn stays_quiet_while_two_feeds_agree() {
    let h = setup();
    let second = h.env.register(MockFeed, ());
    MockFeedClient::new(&h.env, &second).set(&100_000_000_000_000, &NOW, &14);
    h.feed.set(&100_200_000_000_000, &NOW, &14);

    let id = h.register(Trigger::Deviation(FeedDeviation {
        asset: Asset::Other(symbol_short!("USTRY")),
        feeds: vec![&h.env, h.feed_id.clone(), second],
        max_bps: 500,
        max_age: MAX_AGE,
    }));

    assert!(!h.guardian.check(&id));
}

#[test]
fn compares_feeds_that_report_at_different_decimals() {
    let h = setup();
    let second = h.env.register(MockFeed, ());

    // The same $1.00, one feed at 14 decimals and one at 7.
    h.feed.set(&100_000_000_000_000, &NOW, &14);
    MockFeedClient::new(&h.env, &second).set(&10_000_000, &NOW, &7);

    let id = h.register(Trigger::Deviation(FeedDeviation {
        asset: Asset::Other(symbol_short!("USTRY")),
        feeds: vec![&h.env, h.feed_id.clone(), second],
        max_bps: 500,
        max_age: MAX_AGE,
    }));

    assert!(!h.guardian.check(&id));
}

/// The YieldBlox shape, February 22 2026.
///
/// One trade in an illiquid market took USTRY from about $1 to about $106 on the feed the
/// pool trusted. A depositor watching a single feed had nothing to react to, because that
/// feed looked internally consistent the whole way up. A second feed disagreeing by 10,500%
/// is the signal, and it is the one this rule reads.
#[test]
fn a_depositor_exits_when_one_feed_prints_a_manipulated_price() {
    let h = setup();
    let honest = h.env.register(MockFeed, ());

    let manipulated = 10_600_000_000_000_000;
    let sane = 100_000_000_000_000;

    h.feed.set(&sane, &NOW, &14);
    MockFeedClient::new(&h.env, &honest).set(&sane, &NOW, &14);

    let id = h.register(Trigger::Deviation(FeedDeviation {
        asset: Asset::Other(symbol_short!("USTRY")),
        feeds: vec![&h.env, h.feed_id.clone(), honest],
        max_bps: 500,
        max_age: MAX_AGE,
    }));

    assert!(!h.guardian.check(&id), "quiet while the feeds agree");

    h.feed.set(&manipulated, &NOW, &14);

    assert!(h.guardian.check(&id), "the spread is the signal");
    assert_eq!(h.guardian.execute(&id, &h.keeper), 1_000);
    assert_eq!(h.exits(), 1);
}
