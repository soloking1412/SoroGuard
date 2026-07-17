//! The write path: run `execute`, letting the guardian re-derive the trigger itself.
//!
//! The flow is the standard Soroban one. Simulate to learn the resources and the
//! authorization the call needs, fill in the parts only the keeper can provide, sign, send.
//!
//! Two authorizations are in play, and the split is the whole point of SoroGuard:
//!
//! - The root `execute` call is authorized by the keeper, because the keeper is the
//!   transaction source. That is all the authority the keeper has.
//! - The guardian's inner call into the user's position is authorized by the user's guarded
//!   account, which the account grants only against a `Signature::Rule(id)` and only while
//!   the guardian still says the rule holds. The keeper fills that signature in but cannot
//!   forge the authority behind it.

use anyhow::{bail, Context, Result};
use stellar_rpc_client::{AuthMode, Client};
use stellar_xdr::{
    Hash, MuxedAccount, Operation, OperationBody, SequenceNumber, SorobanAuthorizationEntry,
    SorobanCredentials, Transaction, TransactionEnvelope, TransactionExt, TransactionV1Envelope,
    Uint256, VecM,
};

use crate::signer::Signer;
use crate::tx;

/// Submit `guardian.execute(id, keeper)` and return the transaction hash.
pub async fn execute(
    client: &Client,
    signer: &Signer,
    network_passphrase: &str,
    guardian_id: &str,
    id: u32,
    auth_ttl_ledgers: u32,
) -> Result<Hash> {
    let guardian = tx::contract_address(guardian_id)?;
    let keeper_addr = keeper_address(signer)?;
    let source = source_key(signer)?;

    let account = client
        .get_account(&signer.strkey())
        .await
        .context("keeper account not found; is it funded?")?;
    let seq = SequenceNumber(account.seq_num.0 + 1);

    let op = tx::execute_op(&guardian, id, &keeper_addr)?;
    let unsigned = tx::wrap(source.clone(), seq.clone(), tx::BASE_FEE, op);

    // Record mode returns the authorizations the call requires, including the guarded
    // account's, which we then key to this rule.
    let sim = client
        .simulate_transaction_envelope(
            &unsigned_envelope(unsigned.clone()),
            Some(AuthMode::RecordAllowNonRoot),
        )
        .await?;

    if let Some(error) = sim.error {
        bail!("execute simulation failed, trigger likely no longer holds: {error}");
    }

    let expiration = sim.latest_ledger + auth_ttl_ledgers;
    let auth = authorizations(&sim, id, expiration)?;

    let soroban_data = sim
        .transaction_data()
        .context("simulation returned no Soroban transaction data")?;
    let fee = tx::BASE_FEE + u32::try_from(sim.min_resource_fee).unwrap_or(u32::MAX);

    let tx = Transaction {
        source_account: MuxedAccount::Ed25519(source),
        fee,
        seq_num: seq,
        cond: unsigned.cond,
        memo: unsigned.memo,
        operations: with_auth(unsigned.operations.to_vec(), auth)?,
        ext: TransactionExt::V1(soroban_data),
    };

    let signature = signer.sign(network_passphrase, &tx)?;
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: vec![signature].try_into().expect("one signature"),
    });

    Ok(client.send_transaction(&envelope).await?)
}

/// Take the authorization simulation asked for and, for the guarded account, replace the
/// placeholder signature with this rule's.
fn authorizations(
    sim: &stellar_rpc_client::SimulateTransactionResponse,
    id: u32,
    expiration: u32,
) -> Result<Vec<SorobanAuthorizationEntry>> {
    let mut entries = Vec::new();

    for result in sim.results()? {
        for mut entry in result.auth {
            if let SorobanCredentials::Address(creds) = &mut entry.credentials {
                creds.signature_expiration_ledger = expiration;
                creds.signature = tx::rule_signature(id)?;
            }
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn with_auth(
    ops: Vec<Operation>,
    auth: Vec<SorobanAuthorizationEntry>,
) -> Result<VecM<Operation, 100>> {
    let mut op = ops
        .into_iter()
        .next()
        .context("no operation to authorize")?;
    if let OperationBody::InvokeHostFunction(ref mut invoke) = op.body {
        invoke.auth = auth.try_into()?;
    }
    Ok(vec![op].try_into().expect("one operation"))
}

fn keeper_address(signer: &Signer) -> Result<stellar_xdr::ScAddress> {
    Ok(stellar_xdr::ScAddress::Account(signer.account_id()))
}

fn source_key(signer: &Signer) -> Result<Uint256> {
    match signer.muxed_account() {
        MuxedAccount::Ed25519(key) => Ok(key),
        _ => bail!("keeper account must be a plain ed25519 account"),
    }
}

fn unsigned_envelope(tx: Transaction) -> TransactionEnvelope {
    TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: Default::default(),
    })
}
