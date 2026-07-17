use soroban_sdk::{contracttype, Address, Vec};

use crate::oracle::Asset;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub owner: Address,
    pub position: Address,
    pub trigger: Trigger,
    pub action: Action,
    pub cooldown: u64,
    pub last_fired: u64,
    pub status: RuleStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Trigger {
    Price(PriceBelow),
    Health(HealthFloor),
    Deviation(FeedDeviation),
}

/// Fires when the feed's last price drops under `below`.
///
/// `below` is expressed at the oracle's own `decimals()`, so it is only meaningful against
/// the feed named in `oracle`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceBelow {
    pub asset: Asset,
    pub below: i128,
    pub oracle: Address,
    pub max_age: u64,
}

/// Fires when the adapter reports a health factor at or under `floor`.
///
/// `floor` is fixed-point at [`crate::HEALTH_DECIMALS`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthFloor {
    pub adapter: Address,
    pub floor: i128,
}

/// Fires when two feeds for the same asset disagree by more than `max_bps`.
///
/// This is the YieldBlox case. One feed reading a manipulated market prints a price the
/// others don't, and the spread is the signal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedDeviation {
    pub asset: Asset,
    pub feeds: Vec<Address>,
    pub max_bps: u32,
    pub max_age: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    ExitToStable(ExitToStable),
    Repay(Repay),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExitToStable {
    pub to: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repay {
    pub asset: Address,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleStatus {
    Active = 0,
    Cancelled = 1,
}
