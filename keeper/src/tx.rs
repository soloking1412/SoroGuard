//! Assembling the two calls the keeper makes into the guardian.
//!
//! `check` is read-only and needs no signature. `execute` is the same call the guardian
//! runs its trigger re-derivation behind, so a keeper can never make it act on a rule that
//! doesn't currently hold.

use anyhow::Result;
use stellar_xdr::{
    ContractId, Hash, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Memo, Operation,
    OperationBody, Preconditions, ScAddress, ScSymbol, ScVal, ScVec, SequenceNumber, StringM,
    Transaction, TransactionExt, Uint256, VecM,
};

/// Fee in stroops for the non-Soroban part of the transaction. The Soroban resource fee is
/// added on top from what simulation reports.
pub const BASE_FEE: u32 = 100;

pub fn contract_address(id: &str) -> Result<ScAddress> {
    let contract = stellar_strkey::Contract::from_string(id)
        .map_err(|e| anyhow::anyhow!("guardian_id is not a valid C... address: {e}"))?;
    Ok(ScAddress::Contract(ContractId(Hash(contract.0))))
}

fn symbol(name: &str) -> Result<ScSymbol> {
    let inner: StringM<32> = name
        .try_into()
        .map_err(|_| anyhow::anyhow!("function name too long: {name}"))?;
    Ok(ScSymbol(inner))
}

fn invoke_op(host_function: HostFunction) -> Operation {
    Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function,
            auth: VecM::default(),
        }),
    }
}

/// `guardian.check(id)`.
pub fn check_op(guardian: &ScAddress, id: u32) -> Result<Operation> {
    Ok(invoke_op(HostFunction::InvokeContract(
        InvokeContractArgs {
            contract_address: guardian.clone(),
            function_name: symbol("check")?,
            args: vec![ScVal::U32(id)].try_into()?,
        },
    )))
}

/// `guardian.execute(id, keeper)`.
pub fn execute_op(guardian: &ScAddress, id: u32, keeper: &ScAddress) -> Result<Operation> {
    Ok(invoke_op(HostFunction::InvokeContract(
        InvokeContractArgs {
            contract_address: guardian.clone(),
            function_name: symbol("execute")?,
            args: vec![ScVal::U32(id), ScVal::Address(keeper.clone())].try_into()?,
        },
    )))
}

/// The value a guarded account expects as the signature for its keeper path.
///
/// It mirrors the policy contract's `Signature::Rule(id)`. A `#[contracttype]` tuple variant
/// serializes as a two-element vec of the variant name and its payload, so this is exactly
/// what `__check_auth` decodes on the other side.
pub fn rule_signature(id: u32) -> Result<ScVal> {
    let entries: VecM<ScVal> = vec![ScVal::Symbol(symbol("Rule")?), ScVal::U32(id)].try_into()?;
    Ok(ScVal::Vec(Some(ScVec(entries))))
}

/// A single-operation transaction with the given source, sequence and fee.
pub fn wrap(source: Uint256, seq: SequenceNumber, fee: u32, op: Operation) -> Transaction {
    Transaction {
        source_account: stellar_xdr::MuxedAccount::Ed25519(source),
        fee,
        seq_num: seq,
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![op].try_into().expect("one operation"),
        ext: TransactionExt::V0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUARDIAN: &str = "CBQHNAXSI55GX2GN6D67GK7BHVPSLJUGZQEU7WJ5LKR5PNUCGLIMAO4K";

    #[test]
    fn parses_a_contract_address() {
        assert!(matches!(
            contract_address(GUARDIAN).unwrap(),
            ScAddress::Contract(_)
        ));
    }

    #[test]
    fn rejects_a_non_contract_address() {
        assert!(
            contract_address("GDRXE2BQUC3AZNPVFSCEZ76NJ3WWL25FYFK6RGZGIEKWE4SOOHSUJUJ6").is_err()
        );
    }

    #[test]
    fn check_op_carries_the_rule_id() {
        let guardian = contract_address(GUARDIAN).unwrap();
        let op = check_op(&guardian, 7).unwrap();

        let OperationBody::InvokeHostFunction(op) = op.body else {
            panic!("expected an invoke");
        };
        let HostFunction::InvokeContract(call) = op.host_function else {
            panic!("expected a contract call");
        };
        assert_eq!(call.function_name, symbol("check").unwrap());
        assert_eq!(call.args.to_vec(), vec![ScVal::U32(7)]);
    }

    #[test]
    fn rule_signature_matches_the_contracttype_encoding() {
        let ScVal::Vec(Some(ScVec(entries))) = rule_signature(3).unwrap() else {
            panic!("expected a vec");
        };
        assert_eq!(
            entries.to_vec(),
            vec![ScVal::Symbol(symbol("Rule").unwrap()), ScVal::U32(3)]
        );
    }
}
