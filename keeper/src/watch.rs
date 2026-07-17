//! The read path: ask the guardian whether a rule holds, without moving anything.
//!
//! This is a simulation of `check`, so it costs nothing and needs no signature. The keeper
//! only submits a real transaction once this comes back true.

use anyhow::{bail, Result};
use stellar_rpc_client::Client;
use stellar_xdr::{ScVal, Transaction};

use crate::signer::Signer;
use crate::tx;

/// Whether rule `id` on `guardian` would fire right now.
pub async fn rule_holds(
    client: &Client,
    signer: &Signer,
    guardian_id: &str,
    id: u32,
) -> Result<bool> {
    let guardian = tx::contract_address(guardian_id)?;
    let op = tx::check_op(&guardian, id)?;

    // Simulation ignores the sequence and fee, so a source account and zeroes are enough.
    let source = match signer.muxed_account() {
        stellar_xdr::MuxedAccount::Ed25519(key) => key,
        _ => bail!("keeper account must be a plain ed25519 account"),
    };
    let unsigned: Transaction = tx::wrap(source, stellar_xdr::SequenceNumber(0), 0, op);

    let sim = client
        .simulate_transaction_envelope(&envelope(unsigned), None)
        .await?;

    if let Some(error) = sim.error {
        bail!("guardian.check simulation failed: {error}");
    }

    match sim.results()?.into_iter().next() {
        Some(result) => decode_bool(result.xdr),
        None => bail!("guardian.check returned no result"),
    }
}

fn decode_bool(value: ScVal) -> Result<bool> {
    match value {
        ScVal::Bool(b) => Ok(b),
        other => bail!("expected a bool from guardian.check, got {other:?}"),
    }
}

fn envelope(tx: Transaction) -> stellar_xdr::TransactionEnvelope {
    stellar_xdr::TransactionEnvelope::Tx(stellar_xdr::TransactionV1Envelope {
        tx,
        signatures: Default::default(),
    })
}
