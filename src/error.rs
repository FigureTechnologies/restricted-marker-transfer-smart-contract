use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {

    #[error("Insufficient funds to complete the transfer")]
    InsufficientFunds,

    #[error("Invalid fields: {fields:?}")]
    InvalidFields { fields: Vec<String> },

    #[error("Failed to load transfer: {error:?}")]
    LoadTransferFailed { error: StdError },

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("bank sends are not allowed in restricted marker transfers")]
    SentFundsUnsupported,

    #[error("Unauthorized: {error:?}")]
    Unauthorized { error: String },

    #[error("Unsupported upgrade: {source_version:?} => {target_version:?}")]
    UnsupportedUpgrade {
        source_version: String,
        target_version: String,
    },

    #[error("Only restricted markers are supported")]
    UnsupportedMarkerType,
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

// A helper function for creating generic contract errors.
pub fn contract_err(s: &str) -> ContractError {
    ContractError::Std(StdError::generic_err(s))
}
