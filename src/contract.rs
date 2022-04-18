use std::fmt;
use cosmwasm_std::{entry_point};
use cosmwasm_std::{
    attr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary, Uint128
};
use provwasm_std::{
    Marker, MarkerType, ProvenanceMsg, ProvenanceQuerier,
    ProvenanceQuery, transfer_marker_coins,
};

use cw2::set_contract_version;

use crate::error::{contract_err, ContractError};
use crate::msg::{ExecuteMsg, QueryMsg, Validate};
use crate::state::{config, config_read, get_transfer_storage, State, Transfer};

pub const CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

// smart contract execute entrypoint
#[entry_point]
pub fn execute(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    msg.validate()?;

    match msg {
        ExecuteMsg::ApproveTransfer { id } => {
            approve_transfer(deps, env, info, id)
        }
        ExecuteMsg::CancelTransfer { id } => {
            cancel_transfer(deps, env, info, id)
        }
        ExecuteMsg::RejectTransfer { id } => {
            reject_transfer(deps, env, info, id)
        }
        ExecuteMsg::Transfer { id, denom, amount, recipient } => {
            create_transfer(
                deps,
                env,
                info,
                id,
                denom,
                amount,
                recipient,
            )
        }
    }
}

fn create_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
    denom: String,
    amount: Uint128,
    recipient: String
) -> Result<Response<ProvenanceMsg>, ContractError> {

    let transfer = Transfer {
        id,
        sender: info.sender.to_owned(),
        denom,
        amount,
        recipient: deps.api.addr_validate(&recipient)?,
    };

    let is_restricted_marker = matches!(
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(transfer.denom.clone()),
        Ok(Marker {
            marker_type: MarkerType::Restricted,
            ..
        })
    );

    match is_restricted_marker {
        // funds should not be sent
        true => {
            if !info.funds.is_empty() {
                return Err(ContractError::SentFundsUnsupported);
            }
        }
        false => {
            return Err(ContractError::UnsupportedMarkerType);
        }
    }

    // Ensure the sender holds enough denom to cover the transfer.
    let balance = deps
        .querier
        .query_balance(info.sender.clone(), transfer.denom.clone())?;

    if balance.amount < transfer.amount {
        return Err(ContractError::InsufficientFunds);
    }

    let mut transfer_storage = get_transfer_storage(deps.storage);

    if transfer_storage.may_load(transfer.id.as_bytes())?.is_some() {
        return Err(ContractError::InvalidFields {
            fields: vec![String::from("id")],
        });
    }

    transfer_storage.save(transfer.id.as_bytes(), &transfer)?;

    let mut response = Response::new().add_attributes(vec![
        attr("action", Action::Transfer.to_string()),
        attr("id", &transfer.id),
        attr("denom", &transfer.denom),
        attr("amount", &transfer.amount.to_string()),
        attr("sender", &transfer.sender),
        attr("recipient", &transfer.recipient),
    ]);

    response = response.add_message(
        transfer_marker_coins(
            transfer.amount.into(),
            transfer.denom.to_owned(),
            env.contract.address,
            transfer.sender,
        )?
    );

    Ok(response)
}

pub fn cancel_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // implement
    return Err(ContractError::UnsupportedMarkerType);
}

pub fn reject_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // implement
    return Err(ContractError::UnsupportedMarkerType);
}

pub fn approve_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // implement
    return Err(ContractError::UnsupportedMarkerType);
}


// smart contract query entrypoint
// #[entry_point]
// pub fn query(deps: Deps<ProvenanceQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     msg.validate()?;
//
//     // TODO: implement
//
//
//     match msg {
//
//     //     QueryMsg::GetAsk { id } => {
//     //         let ask_storage_read = get_ask_storage_read(deps.storage);
//     //         return to_binary(&ask_storage_read.load(id.as_bytes())?);
//     //     }
//     //     QueryMsg::GetBid { id } => {
//     //         let bid_storage_read = get_bid_storage_read(deps.storage);
//     //         return to_binary(&bid_storage_read.load(id.as_bytes())?);
//     //     }
//     //     QueryMsg::GetContractInfo {} => to_binary(&get_contract_info(deps.storage)?),
//     //     QueryMsg::GetVersionInfo {} => to_binary(&get_version_info(deps.storage)?),
//
//         QueryMsg::GetContractInfo { } => to_binary(&config_read(deps.storage).load()),
//         QueryMsg::GetVersionInfo { } => to_binary(&config_read(deps.storage).load()),
//         QueryMsg::GetTransfer { .. } => to_binary(&config_read(deps.storage).load()),
//         QueryMsg::GetAllTransfers { .. } => to_binary(&config_read(deps.storage).load()),
//     }
// }

enum Action {
    Transfer,
    Approve,
    Reject,
    Cancel
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::Transfer => write!(f, "create_transfer"),
            Action::Approve => write!(f, "approve"),
            Action::Reject => write!(f, "reject"),
            Action::Cancel => write!(f, "cancel"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Addr, Storage, coin};
    use provwasm_mocks::mock_dependencies;
    use crate::state::get_transfer_storage_read;

    const RESTRICTED_DENOM: &str = "restricted_1";

    #[test]
    fn create_transfer_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let test_marker: Marker = setup_restricted_marker();
        deps.querier.with_markers(vec![test_marker]);

        let transfer_id = "56253028-12f5-4d2a-a691-ebdfd2a7b865";
        let amount = Uint128::new(1);
        let transfer_msg = ExecuteMsg::Transfer {
            id: transfer_id.into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_info = mock_info("sender", &[]);

        let sender_balance = coin(1, RESTRICTED_DENOM);
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        let recipient = "transfer_to";

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                assert_eq!(response.attributes.len(), 6);
                assert_eq!(response.attributes[0], attr("action", Action::Transfer.to_string()));
                assert_eq!(response.attributes[1], attr("id", transfer_id));
                assert_eq!(response.attributes[2], attr("denom", RESTRICTED_DENOM));
                assert_eq!(response.attributes[3], attr("amount", amount.to_string()));
                assert_eq!(response.attributes[4], attr("sender", sender_info.clone().sender));
                assert_eq!(response.attributes[5], attr("recipient", recipient));

                assert_eq!(response.messages.len(), 1);
                assert_eq!(
                    response.messages[0].msg,
                    transfer_marker_coins(
                        amount.u128(),
                        RESTRICTED_DENOM.to_owned(),
                        Addr::unchecked(MOCK_CONTRACT_ADDR),
                        sender_info.clone().sender
                    )
                        .unwrap()
                );
            }
            Err(error) => {
                panic!("failed to create transfer: {:?}", error)
            }
        }

        // verify transfer stored
        let transfer_storage = get_transfer_storage_read(&deps.storage);

        match transfer_storage.load(transfer_id.as_bytes()) {
            Ok(stored_transfer) => {
                assert_eq!(
                    stored_transfer,
                    Transfer {
                        id: transfer_id.into(),
                        sender: sender_info.sender.to_owned(),
                        denom: RESTRICTED_DENOM.into(),
                        amount,
                        recipient: Addr::unchecked(recipient)
                    }
                )
            }
            _ => {
                panic!("transfer was not found in storage")
            }
        }
    }

    #[test]
    fn create_transfer_with_funds_throws_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let test_marker: Marker = setup_restricted_marker();
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(1);
        let transfer_msg = ExecuteMsg::Transfer {
            id: "56253028-12f5-4d2a-a691-ebdfd2a7b865".into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_info = mock_info(
            "sender",
            &[coin(amount.u128(), RESTRICTED_DENOM)]
        );

        let sender_balance = coin(1, RESTRICTED_DENOM);
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                panic!("expected error, but ok")
            }
            Err(error) => match error {
                ContractError::SentFundsUnsupported => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn create_transfer_insufficient_funds_throws_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let test_marker: Marker = setup_restricted_marker();
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(2);
        let transfer_msg = ExecuteMsg::Transfer {
            id: "56253028-12f5-4d2a-a691-ebdfd2a7b865".into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_info = mock_info("sender", &[]);

        let sender_balance = coin(1, RESTRICTED_DENOM);
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                panic!("expected error, but ok")
            }
            Err(error) => match error {
                ContractError::InsufficientFunds => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn create_transfer_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let test_marker: Marker = setup_restricted_marker();
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(1);
        let transfer_msg = ExecuteMsg::Transfer {
            id: "".into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_info = mock_info("sender", &[]);

        let sender_balance = coin(1, RESTRICTED_DENOM);
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                panic!("expected error, but ok")
            }
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert!(fields.contains(&"id".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn create_transfer_existing_id() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let test_marker: Marker = setup_restricted_marker();
        deps.querier.with_markers(vec![test_marker]);

        let transfer_id = "56253028-12f5-4d2a-a691-ebdfd2a7b865";
        let amount = Uint128::new(1);
        let sender_info = mock_info("sender", &[]);

        store_test_transfer(&mut deps.storage, &Transfer {
            id: transfer_id.into(),
            sender: sender_info.sender.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: Addr::unchecked("transfer_to"),
        });

        let transfer_msg = ExecuteMsg::Transfer {
            id: transfer_id.into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_balance = coin(1, RESTRICTED_DENOM);
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                panic!("expected error, but ok")
            }
            Err(error) => match error {
                ContractError::InvalidFields { fields } => {
                    assert!(fields.contains(&"id".into()));
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn create_transfer_unrestricted_marker_throws_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let amount = Uint128::new(1);
        let transfer_msg = ExecuteMsg::Transfer {
            id: "56253028-12f5-4d2a-a691-ebdfd2a7b865".into(),
            denom: "unrestricted-marker".into(),
            amount: amount.into(),
            recipient: "transfer_to".into()
        };

        let sender_info = mock_info("sender", &[]);

        let sender_balance = coin(amount.u128(), "unrestricted-marker");
        deps.querier
            .base
            .update_balance(Addr::unchecked("sender"), vec![sender_balance]);

        // execute create transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            transfer_msg.clone(),
        );

        // verify transfer response
        match transfer_response {
            Ok(response) => {
                panic!("expected error, but ok")
            }
            Err(error) =>  match error {
                ContractError::UnsupportedMarkerType => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    fn setup_test_base(storage: &mut dyn Storage, contract_info: &State) {
        if let Err(error) = config(storage).save(&contract_info) {
            panic!("unexpected error: {:?}", error)
        }
    }

    fn store_test_transfer(storage: &mut dyn Storage, transfer: &Transfer) {
        let mut transfer_storage = get_transfer_storage(storage);
        if let Err(error) = transfer_storage.save(transfer.id.as_bytes(), transfer) {
            panic!("unexpected error: {:?}", error)
        };
    }

    fn setup_restricted_marker() -> Marker {
        let marker_json = b"{
              \"address\": \"tp1l330sxue4suxz9dhc40e2pns0ymrytf8uz4squ\",
              \"coins\": [
                {
                  \"denom\": \"restricted_1\",
                  \"amount\": \"1000\"
                }
              ],
              \"account_number\": 10,
              \"sequence\": 0,
              \"permissions\": [
                {
                  \"permissions\": [
                    \"burn\",
                    \"delete\",
                    \"deposit\",
                    \"admin\",
                    \"mint\",
                    \"withdraw\"
                  ],
                  \"address\": \"tp13pnzut8zdjaqht7aqe7kk4ww5zfq04jzlytnmu\"
                }
              ],
              \"status\": \"active\",
              \"denom\": \"restricted_1\",
              \"total_supply\": \"1000\",
              \"marker_type\": \"restricted\",
              \"supply_fixed\": false
            }";

        return from_binary(&Binary::from(marker_json)).unwrap();
    }
}
