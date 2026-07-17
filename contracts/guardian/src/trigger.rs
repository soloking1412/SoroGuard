//! Re-derivation of a rule's trigger from live on-chain state.
//!
//! Nothing here reads a number supplied by a caller. Every price comes from the feed the
//! rule itself names. That is the whole reason a keeper can't lie about a trigger.

use soroban_sdk::{Address, Env};

use crate::adapter::HealthClient;
use crate::error::Error;
use crate::oracle::{Asset, PriceFeedClient};
use crate::rule::{FeedDeviation, HealthFloor, PriceBelow, Trigger};

const BPS: i128 = 10_000;

/// Common scale for comparing feeds that report at different `decimals()`.
const NORM_DECIMALS: u32 = 14;

pub fn holds(env: &Env, owner: &Address, trigger: &Trigger) -> Result<bool, Error> {
    match trigger {
        Trigger::Price(t) => price_below(env, t),
        Trigger::Health(t) => health_at_floor(env, owner, t),
        Trigger::Deviation(t) => feeds_deviate(env, t),
    }
}

fn price_below(env: &Env, t: &PriceBelow) -> Result<bool, Error> {
    let feed = PriceFeedClient::new(env, &t.oracle);
    let price = fresh_price(env, &feed, &t.asset, t.max_age)?;
    Ok(price < t.below)
}

fn health_at_floor(env: &Env, owner: &Address, t: &HealthFloor) -> Result<bool, Error> {
    let health = HealthClient::new(env, &t.adapter).health_factor(owner);
    Ok(health <= t.floor)
}

/// True when the widest pair of feeds disagrees by more than `max_bps`.
///
/// The spread is taken against the lowest feed, which reports a larger deviation than
/// taking it against the mean would. For a rule whose job is getting a user out, erring
/// toward firing is the right direction to err in.
fn feeds_deviate(env: &Env, t: &FeedDeviation) -> Result<bool, Error> {
    if t.feeds.len() < 2 {
        return Err(Error::NotEnoughFeeds);
    }

    let mut low = i128::MAX;
    let mut high = i128::MIN;

    for oracle in t.feeds.iter() {
        let price = normalized_price(env, &oracle, &t.asset, t.max_age)?;
        if price < low {
            low = price;
        }
        if price > high {
            high = price;
        }
    }

    match (high - low).checked_mul(BPS) {
        Some(scaled) => Ok(scaled / low > i128::from(t.max_bps)),
        // A spread that overflows i128 when scaled to bps exceeds any threshold expressible
        // in a u32.
        None => Ok(true),
    }
}

fn normalized_price(
    env: &Env,
    oracle: &Address,
    asset: &Asset,
    max_age: u64,
) -> Result<i128, Error> {
    let feed = PriceFeedClient::new(env, oracle);
    let price = fresh_price(env, &feed, asset, max_age)?;
    rescale(price, feed.decimals(), NORM_DECIMALS)
}

/// The feed's last price, rejected if it is missing, stale, or not positive.
fn fresh_price(
    env: &Env,
    feed: &PriceFeedClient,
    asset: &Asset,
    max_age: u64,
) -> Result<i128, Error> {
    let data = feed.lastprice(asset).ok_or(Error::OracleUnavailable)?;

    if env.ledger().timestamp().saturating_sub(data.timestamp) > max_age {
        return Err(Error::OracleStale);
    }
    if data.price <= 0 {
        return Err(Error::BadFeedPrice);
    }

    Ok(data.price)
}

fn rescale(price: i128, from: u32, to: u32) -> Result<i128, Error> {
    if from == to {
        return Ok(price);
    }
    if from < to {
        let factor = 10i128.checked_pow(to - from).ok_or(Error::BadFeedPrice)?;
        price.checked_mul(factor).ok_or(Error::BadFeedPrice)
    } else {
        let factor = 10i128.checked_pow(from - to).ok_or(Error::BadFeedPrice)?;
        Ok(price / factor)
    }
}
