use std::fmt;
use crate::error::{contract_err, ContractError};
use cosmwasm_std::{Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
}

/// Simple validation of InstantiateMsg data
///
/// ### Example
///
/// ```rust
/// use restricted_marker_transfer::msg::{InstantiateMsg, Validate};
/// pub fn instantiate(msg: InstantiateMsg){
///
///     let result = msg.validate();
/// }
/// ```
impl Validate for InstantiateMsg {
    fn validate(&self) -> Result<(), ContractError> {
        let mut invalid_fields: Vec<&str> = vec![];

        if self.name.is_empty() {
            invalid_fields.push("name");
        }

        match invalid_fields.len() {
            0 => Ok(()),
            _ => Err(ContractError::InvalidFields {
                fields: invalid_fields.into_iter().map(|item| item.into()).collect(),
            }),
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ApproveTransfer {
        id: String,
    },
    CancelTransfer {
        id: String,
    },
    RejectTransfer {
        id: String,
    },
    Transfer {
        id: String,
        denom: String,
        amount: Uint128,
        recipient: String,
    },
}

impl Validate for ExecuteMsg {
    /// Simple validation of ExecuteMsg data
    ///
    /// ### Example
    ///
    /// ```rust
    /// use restricted_marker_transfer::msg::{ExecuteMsg, Validate};
    ///
    /// pub fn execute(msg: ExecuteMsg){
    ///     let result = msg.validate();
    ///     todo!()
    /// }
    /// ```
    fn validate(&self) -> Result<(), ContractError> {
        let mut invalid_fields: Vec<&str> = vec![];

        match self {
            // TODO: implement
            ExecuteMsg::ApproveTransfer { id } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }
            }
            ExecuteMsg::CancelTransfer { .. } => {}
            ExecuteMsg::RejectTransfer { .. } => {}
            ExecuteMsg::Transfer {
                id,
                denom,
                amount,
                recipient
            } => {

                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }

                // Ensure amount is non-zero.
                if amount.is_zero() {
                    return Err(contract_err("invalid transfer amount"));
                }


            } // validate id, to address is an address
        }

        match invalid_fields.len() {
            0 => Ok(()),
            _ => Err(ContractError::InvalidFields {
                fields: invalid_fields.into_iter().map(|item| item.into()).collect(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTransfer { id: String },
    GetAllTransfers { denom: String },
    GetContractInfo {},
    GetVersionInfo {},
}

impl Validate for QueryMsg {
    /// Simple validation of QueryMsg data
    ///
    /// ### Example
    ///
    /// ```rust
    /// use restricted_marker_transfer::msg::{QueryMsg, Validate};
    /// pub fn query(msg: QueryMsg){
    ///
    ///     let result = msg.validate();
    /// }
    /// ```
    fn validate(&self) -> Result<(), ContractError> {
        let mut invalid_fields: Vec<&str> = vec![];

        match self {
            QueryMsg::GetTransfer { id } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }
            }
            QueryMsg::GetAllTransfers { denom } => {
                if denom.is_empty() {
                    invalid_fields.push("denom");
                }
            }
            QueryMsg::GetContractInfo {} => {}
            QueryMsg::GetVersionInfo {} => {}
        }

        match invalid_fields.len() {
            0 => Ok(()),
            _ => Err(ContractError::InvalidFields {
                fields: invalid_fields.into_iter().map(|item| item.into()).collect(),
            }),
        }
    }
}

// TODO: migrate message

// TODO: query response
// We define a custom struct for each query response
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct CountResponse {
//     pub count: i32,
// }


pub trait Validate {
    fn validate(&self) -> Result<(), ContractError>;
}
