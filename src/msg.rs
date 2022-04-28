use crate::error::ContractError;
use cosmwasm_std::Uint128;
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
pub struct MigrateMsg {}

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
            ExecuteMsg::ApproveTransfer { id } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }
            }
            ExecuteMsg::CancelTransfer { id } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }
            }
            ExecuteMsg::RejectTransfer { id } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }
            }
            ExecuteMsg::Transfer {
                id,
                denom,
                amount,
                recipient,
            } => {
                if Uuid::parse_str(id).is_err() {
                    invalid_fields.push("id");
                }

                if amount.lt(&Uint128::new(1)) {
                    invalid_fields.push("amount");
                }
                if denom.is_empty() {
                    invalid_fields.push("denom");
                }
                if recipient.is_empty() {
                    invalid_fields.push("recipient");
                }
            }
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

pub trait Validate {
    fn validate(&self) -> Result<(), ContractError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ExecuteMsg::{ApproveTransfer, CancelTransfer, RejectTransfer, Transfer};

    #[test]
    fn validate_transfer() {
        let invalid_transfer_msg = Transfer {
            id: "fake-id".to_string(),
            denom: "".to_string(),
            amount: Uint128::new(0),
            recipient: "".to_string(),
        };

        let validate_response = invalid_transfer_msg.validate();

        match validate_response {
            Ok(..) => panic!("expected error but was ok"),
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert_eq!(4, fields.len());
                    assert!(fields.contains(&"id".into()));
                    assert!(fields.contains(&"denom".into()));
                    assert!(fields.contains(&"amount".into()));
                    assert!(fields.contains(&"recipient".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn validate_approve_transfer() {
        let invalid_approve_msg = ApproveTransfer {
            id: "not-a-real-uuid".to_string(),
        };

        let validate_response = invalid_approve_msg.validate();

        match validate_response {
            Ok(..) => panic!("expected error but was ok"),
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert_eq!(1, fields.len());
                    assert!(fields.contains(&"id".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn validate_cancel_transfer() {
        let invalid_cancel_msg = CancelTransfer {
            id: "not-a-real-uuid".to_string(),
        };

        let validate_response = invalid_cancel_msg.validate();

        match validate_response {
            Ok(..) => panic!("expected error but was ok"),
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert_eq!(1, fields.len());
                    assert!(fields.contains(&"id".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn validate_reject_transfer() {
        let invalid_reject_msg = RejectTransfer {
            id: "not-a-real-uuid".to_string(),
        };

        let validate_response = invalid_reject_msg.validate();

        match validate_response {
            Ok(..) => panic!("expected error but was ok"),
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert_eq!(1, fields.len());
                    assert!(fields.contains(&"id".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }
}
