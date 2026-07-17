use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    RuleNotFound = 1,
    NotOwner = 2,
    RuleInactive = 3,
    /// The rule fired too recently. Guards against acting on a single-ledger wick.
    CooldownActive = 4,
    /// The trigger did not hold when the guardian re-derived it.
    TriggerNotMet = 5,
    /// The feed answered, but with a price old enough that acting on it is unsafe.
    OracleStale = 6,
    /// The feed had no price for the asset at all.
    OracleUnavailable = 7,
    CooldownTooShort = 8,
    /// A deviation trigger needs at least two feeds to compare.
    NotEnoughFeeds = 9,
    NonPositiveThreshold = 10,
    /// A feed reported a price at or below zero.
    BadFeedPrice = 11,
}
