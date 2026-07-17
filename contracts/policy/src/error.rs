use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    /// The owner never armed this rule, or has since revoked it.
    RuleNotArmed = 2,
    /// The guardian re-derived the trigger and it does not hold.
    RuleNotTriggered = 3,
    /// The call targets a contract this rule was not armed against.
    ContractNotAllowed = 4,
    /// The call targets a function this rule was not armed against.
    FunctionNotAllowed = 5,
    NoContexts = 6,
}
