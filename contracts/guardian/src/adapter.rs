//! Interfaces SoroGuard expects of the protocol it is pulling a user out of.
//!
//! Adapters are ordinary contracts that translate these calls into whatever the underlying
//! protocol already exposes publicly. Anyone can deploy one. Blend, Soroswap and Aquarius
//! don't have to agree to anything, or even know SoroGuard exists.

use soroban_sdk::{contractclient, Address, Env};

#[contractclient(name = "PositionClient")]
pub trait PositionAdapter {
    /// Unwind `owner`'s position and settle the proceeds into `to`. Returns the amount out.
    ///
    /// Requires `owner`'s authorization, which is what routes the call through the user's
    /// smart account and therefore through their policy.
    fn exit_to(env: Env, owner: Address, to: Address) -> i128;

    /// Repay `owner`'s borrow of `asset`. Returns the amount repaid.
    fn repay(env: Env, owner: Address, asset: Address) -> i128;
}

#[contractclient(name = "HealthClient")]
pub trait HealthAdapter {
    /// `owner`'s health factor, fixed-point at [`crate::HEALTH_DECIMALS`].
    ///
    /// Below 1.0 means liquidatable on every lending market this targets.
    fn health_factor(env: Env, owner: Address) -> i128;
}
