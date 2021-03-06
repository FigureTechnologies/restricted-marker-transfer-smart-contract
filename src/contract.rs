use std::fmt;

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cosmwasm_std::{entry_point, Addr};
use provwasm_std::{
    transfer_marker_coins, Marker, MarkerAccess, MarkerType, ProvenanceMsg, ProvenanceQuerier,
    ProvenanceQuery,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, QueryMsg, Validate};
use crate::state::{config_read, get_transfer_storage, get_transfer_storage_read, Transfer};

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
        ExecuteMsg::ApproveTransfer { id } => approve_transfer(deps, env, info, id),
        ExecuteMsg::CancelTransfer { id } => cancel_transfer(deps, env, info, id),
        ExecuteMsg::RejectTransfer { id } => reject_transfer(deps, env, info, id),
        ExecuteMsg::Transfer {
            id,
            denom,
            amount,
            recipient,
        } => create_transfer(deps, env, info, id, denom, amount, recipient),
    }
}

fn create_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
    denom: String,
    amount: Uint128,
    recipient: String,
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

    response = response.add_message(transfer_marker_coins(
        transfer.amount.into(),
        transfer.denom.to_owned(),
        env.contract.address,
        transfer.sender,
    )?);

    Ok(response)
}

pub fn cancel_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    transfer_id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let transfer_storage_read = get_transfer_storage_read(deps.storage);
    let transfer = transfer_storage_read
        .load(transfer_id.as_bytes())
        .map_err(|error| ContractError::LoadTransferFailed { error })?;

    if !info.funds.is_empty() {
        return Err(ContractError::SentFundsUnsupported);
    }

    if !info.sender.eq(&transfer.sender) {
        return Err(ContractError::Unauthorized {
            error: String::from("Only original sender can cancel"),
        });
    }

    let mut response = Response::new().add_attributes(vec![
        attr("action", Action::Cancel.to_string()),
        attr("id", &transfer.id),
        attr("denom", &transfer.denom),
        attr("amount", &transfer.amount.to_string()),
        attr("sender", &transfer.sender),
    ]);

    response = response.add_message(transfer_marker_coins(
        transfer.amount.into(),
        transfer.denom.to_owned(),
        transfer.sender,
        env.contract.address,
    )?);

    // finally remove the transfer from storage
    get_transfer_storage(deps.storage).remove(transfer_id.as_bytes());

    Ok(response)
}

pub fn reject_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    transfer_id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let transfer_storage_read = get_transfer_storage_read(deps.storage);
    let transfer = transfer_storage_read
        .load(transfer_id.as_bytes())
        .map_err(|error| ContractError::LoadTransferFailed { error })?;

    if !info.funds.is_empty() {
        return Err(ContractError::SentFundsUnsupported);
    }

    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(transfer.denom.clone())?;

    if !is_marker_admin(info.sender.to_owned(), marker) {
        return Err(ContractError::Unauthorized {
            error: String::from("MARKER_ADMIN permission is required to reject transfers"),
        });
    }

    let mut response = Response::new().add_attributes(vec![
        attr("action", Action::Reject.to_string()),
        attr("id", &transfer.id),
        attr("denom", &transfer.denom),
        attr("amount", &transfer.amount.to_string()),
        attr("sender", &transfer.sender),
        attr("admin", info.sender.to_owned()),
    ]);

    response = response.add_message(transfer_marker_coins(
        transfer.amount.into(),
        transfer.denom.to_owned(),
        transfer.sender,
        env.contract.address,
    )?);

    // finally remove the transfer from storage
    get_transfer_storage(deps.storage).remove(transfer_id.as_bytes());

    Ok(response)
}

pub fn approve_transfer(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    transfer_id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let transfer_storage_read = get_transfer_storage_read(deps.storage);
    let transfer = transfer_storage_read
        .load(transfer_id.as_bytes())
        .map_err(|error| ContractError::LoadTransferFailed { error })?;

    if !info.funds.is_empty() {
        return Err(ContractError::SentFundsUnsupported);
    }

    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(transfer.denom.clone())?;

    if !is_marker_admin(info.sender.to_owned(), marker) {
        return Err(ContractError::Unauthorized {
            error: String::from("MARKER_ADMIN permission is required to approve transfers"),
        });
    }

    let mut response = Response::new().add_attributes(vec![
        attr("action", Action::Approve.to_string()),
        attr("id", &transfer.id),
        attr("denom", &transfer.denom),
        attr("amount", &transfer.amount.to_string()),
        attr("sender", &transfer.sender),
        attr("recipient", &transfer.recipient),
        attr("admin", &info.sender),
    ]);

    response = response.add_message(transfer_marker_coins(
        transfer.amount.into(),
        transfer.denom.to_owned(),
        transfer.recipient.to_owned(),
        env.contract.address.to_owned(),
    )?);

    // finally remove the transfer from storage
    get_transfer_storage(deps.storage).remove(transfer_id.as_bytes());
    Ok(response)
}

/// returns true if the sender has marker admin permissions for the given marker
fn is_marker_admin(sender: Addr, marker: Marker) -> bool {
    marker.permissions.iter().any(|grant| {
        grant.address == sender
            && grant
                .permissions
                .iter()
                .any(|marker_access| matches!(marker_access, MarkerAccess::Admin))
    })
}

#[entry_point]
pub fn query(deps: Deps<ProvenanceQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    msg.validate()?;

    match msg {
        QueryMsg::GetContractInfo {} => to_binary(&config_read(deps.storage).load()?),
        QueryMsg::GetVersionInfo {} => to_binary(&cw2::get_contract_version(deps.storage)?),
        QueryMsg::GetTransfer { id: transfer_id } => {
            to_binary(&get_transfer_storage_read(deps.storage).load(transfer_id.as_bytes())?)
        }
    }
}

enum Action {
    Transfer,
    Approve,
    Reject,
    Cancel,
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
    use crate::state::{config, State};
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, from_binary, Addr, Storage};
    use provwasm_mocks::mock_dependencies;

    use crate::state::get_transfer_storage_read;

    use super::*;

    const RESTRICTED_DENOM: &str = "restricted_1";
    const TRANSFER_ID: &str = "56253028-12f5-4d2a-a691-ebdfd2a7b865";

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

        let amount = Uint128::new(1);
        let transfer_msg = ExecuteMsg::Transfer {
            id: TRANSFER_ID.into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into(),
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
                assert_eq!(
                    response.attributes[0],
                    attr("action", Action::Transfer.to_string())
                );
                assert_eq!(response.attributes[1], attr("id", TRANSFER_ID));
                assert_eq!(response.attributes[2], attr("denom", RESTRICTED_DENOM));
                assert_eq!(response.attributes[3], attr("amount", amount.to_string()));
                assert_eq!(
                    response.attributes[4],
                    attr("sender", sender_info.clone().sender)
                );
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

        match transfer_storage.load(TRANSFER_ID.as_bytes()) {
            Ok(stored_transfer) => {
                assert_eq!(
                    stored_transfer,
                    Transfer {
                        id: TRANSFER_ID.into(),
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
            recipient: "transfer_to".into(),
        };

        let sender_info = mock_info("sender", &[coin(amount.u128(), RESTRICTED_DENOM)]);

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

        assert_sent_funds_unsupported_error(transfer_response);
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
            id: TRANSFER_ID.into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into(),
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
            Ok(..) => {
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
            recipient: "transfer_to".into(),
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
            Ok(..) => panic!("expected error, but ok"),
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

        let amount = Uint128::new(1);
        let sender_info = mock_info("sender", &[]);

        store_test_transfer(
            &mut deps.storage,
            &Transfer {
                id: TRANSFER_ID.into(),
                sender: sender_info.sender.to_owned(),
                denom: RESTRICTED_DENOM.into(),
                amount,
                recipient: Addr::unchecked("transfer_to"),
            },
        );

        let transfer_msg = ExecuteMsg::Transfer {
            id: TRANSFER_ID.into(),
            denom: RESTRICTED_DENOM.into(),
            amount: amount.into(),
            recipient: "transfer_to".into(),
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
            Ok(..) => panic!("expected error, but ok"),
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
            id: TRANSFER_ID.into(),
            denom: "unrestricted-marker".into(),
            amount: amount.into(),
            recipient: "transfer_to".into(),
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
            Ok(..) => panic!("expected error, but ok"),
            Err(error) => match error {
                ContractError::UnsupportedMarkerType => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn approve_transfer_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let admin_address = Addr::unchecked("admin_address");
        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(1);
        let sender_info = mock_info(admin_address.as_str(), &[]);

        store_test_transfer(
            &mut deps.storage,
            &Transfer {
                id: TRANSFER_ID.into(),
                sender: sender_address.to_owned(),
                denom: RESTRICTED_DENOM.into(),
                amount,
                recipient: recipient_address.to_owned(),
            },
        );

        let approve_transfer_msg = ExecuteMsg::ApproveTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute approve transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            approve_transfer_msg.clone(),
        );

        // verify approve transfer response
        match transfer_response {
            Ok(response) => {
                assert_eq!(response.attributes.len(), 7);
                assert_eq!(
                    response.attributes[0],
                    attr("action", Action::Approve.to_string())
                );
                assert_eq!(response.attributes[1], attr("id", TRANSFER_ID));
                assert_eq!(response.attributes[2], attr("denom", RESTRICTED_DENOM));
                assert_eq!(response.attributes[3], attr("amount", amount.to_string()));
                assert_eq!(response.attributes[4], attr("sender", sender_address));
                assert_eq!(
                    response.attributes[5],
                    attr("recipient", recipient_address.to_owned())
                );
                assert_eq!(response.attributes[6], attr("admin", admin_address));

                assert_eq!(response.messages.len(), 1);
                assert_eq!(
                    response.messages[0].msg,
                    transfer_marker_coins(
                        amount.u128(),
                        RESTRICTED_DENOM.to_owned(),
                        recipient_address,
                        Addr::unchecked(MOCK_CONTRACT_ADDR)
                    )
                    .unwrap()
                );
            }
            Err(error) => {
                panic!("failed to create transfer: {:?}", error)
            }
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            None,
            transfer_storage.may_load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn approve_transfer_sent_funds_returns_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let admin_address = Addr::unchecked("admin_address");
        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(1);
        let sender_info = mock_info(admin_address.as_str(), &[coin(1, RESTRICTED_DENOM)]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let approve_transfer_msg = ExecuteMsg::ApproveTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute approve transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            approve_transfer_msg.clone(),
        );

        // verify approve transfer response
        assert_sent_funds_unsupported_error(transfer_response);

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn approve_transfer_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let admin_address = Addr::unchecked("admin_address");
        let approver_address = Addr::unchecked("approver_address");
        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(1);
        let sender_info = mock_info(approver_address.as_str(), &[]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let approve_transfer_msg = ExecuteMsg::ApproveTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute approve transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            approve_transfer_msg.clone(),
        );

        match transfer_response {
            Ok(..) => {
                panic!("expected error, but ok")
            }
            Err(error) => match error {
                ContractError::Unauthorized { .. } => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn approve_transfer_unknown_transfer() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let admin_address = Addr::unchecked("admin_address");
        let sender_info = mock_info(admin_address.as_str(), &[]);

        let approve_transfer_msg = ExecuteMsg::ApproveTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute approve transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            approve_transfer_msg.clone(),
        );

        assert_load_transfer_error(transfer_response);
    }

    #[test]
    fn is_marker_admin_success() {
        let admin_address = Addr::unchecked("admin_address");
        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        assert!(is_marker_admin(
            admin_address.to_owned(),
            test_marker.into()
        ))
    }

    #[test]
    fn is_marker_admin_returns_false_with_no_permission() {
        let admin_address = Addr::unchecked("admin_address");
        let other_address = Addr::unchecked("other_address");
        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        assert_eq!(
            false,
            is_marker_admin(other_address.to_owned(), test_marker.into())
        )
    }

    #[test]
    fn is_marker_admin_returns_false_without_admin_permission() {
        let non_admin_address = Addr::unchecked("some_address_without_admin");
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
                    \"transfer\"
                  ],
                  \"address\": \"some_address_without_admin\"
                }
              ],
              \"status\": \"active\",
              \"denom\": \"restricted_1\",
              \"total_supply\": \"1000\",
              \"marker_type\": \"restricted\",
              \"supply_fixed\": false
            }";

        let test_marker: Marker = from_binary(&Binary::from(marker_json)).unwrap();

        assert_eq!(
            false,
            is_marker_admin(non_admin_address.to_owned(), test_marker.into())
        )
    }

    #[test]
    fn cancel_transfer_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let amount = Uint128::new(3);
        let sender_info = mock_info(sender_address.as_str(), &[]);

        store_test_transfer(
            &mut deps.storage,
            &Transfer {
                id: TRANSFER_ID.into(),
                sender: sender_address.to_owned(),
                denom: RESTRICTED_DENOM.into(),
                amount,
                recipient: recipient_address.to_owned(),
            },
        );

        let cancel_transfer_msg = ExecuteMsg::CancelTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute cancel transfer
        let cancel_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            cancel_transfer_msg.clone(),
        );

        // verify approve transfer response
        match cancel_response {
            Ok(response) => {
                assert_eq!(response.attributes.len(), 5);
                assert_eq!(
                    response.attributes[0],
                    attr("action", Action::Cancel.to_string())
                );
                assert_eq!(response.attributes[1], attr("id", TRANSFER_ID));
                assert_eq!(response.attributes[2], attr("denom", RESTRICTED_DENOM));
                assert_eq!(response.attributes[3], attr("amount", amount.to_string()));
                assert_eq!(
                    response.attributes[4],
                    attr("sender", sender_address.to_owned())
                );

                assert_eq!(response.messages.len(), 1);
                assert_eq!(
                    response.messages[0].msg,
                    transfer_marker_coins(
                        amount.u128(),
                        RESTRICTED_DENOM.to_owned(),
                        sender_address.to_owned(),
                        Addr::unchecked(MOCK_CONTRACT_ADDR)
                    )
                    .unwrap()
                );
            }
            Err(error) => {
                panic!("failed to cancel transfer: {:?}", error)
            }
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            None,
            transfer_storage.may_load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn cancel_transfer_sent_funds_returns_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let amount = Uint128::new(3);
        let sender_info = mock_info(sender_address.as_str(), &[coin(1, RESTRICTED_DENOM)]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let cancel_transfer_msg = ExecuteMsg::CancelTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute cancel transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            cancel_transfer_msg.clone(),
        );

        // verify cancel transfer response
        assert_sent_funds_unsupported_error(transfer_response);

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn cancel_transfer_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let amount = Uint128::new(3);
        let sender_info = mock_info(&"other_address".to_string(), &[]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let cancel_transfer_msg = ExecuteMsg::CancelTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute cancel transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            cancel_transfer_msg.clone(),
        );

        // verify cancel transfer response
        match transfer_response {
            Ok(..) => panic!("expected error, but ok"),
            Err(error) => match error {
                ContractError::Unauthorized { .. } => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn cancel_transfer_unknown_transfer() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let sender_info = mock_info(sender_address.as_str(), &[]);

        let reject_transfer_msg = ExecuteMsg::CancelTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute cancel transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            reject_transfer_msg.clone(),
        );

        assert_load_transfer_error(transfer_response);
    }

    #[test]
    fn reject_transfer_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let admin_address = Addr::unchecked("admin_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(3);
        let sender_info = mock_info(admin_address.as_str(), &[]);

        store_test_transfer(
            &mut deps.storage,
            &Transfer {
                id: TRANSFER_ID.into(),
                sender: sender_address.to_owned(),
                denom: RESTRICTED_DENOM.into(),
                amount,
                recipient: recipient_address.to_owned(),
            },
        );

        let reject_transfer_msg = ExecuteMsg::RejectTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute reject transfer
        let reject_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            reject_transfer_msg.clone(),
        );

        // verify approve transfer response
        match reject_response {
            Ok(response) => {
                assert_eq!(response.attributes.len(), 6);
                assert_eq!(
                    response.attributes[0],
                    attr("action", Action::Reject.to_string())
                );
                assert_eq!(response.attributes[1], attr("id", TRANSFER_ID));
                assert_eq!(response.attributes[2], attr("denom", RESTRICTED_DENOM));
                assert_eq!(response.attributes[3], attr("amount", amount.to_string()));
                assert_eq!(
                    response.attributes[4],
                    attr("sender", sender_address.to_owned())
                );
                assert_eq!(
                    response.attributes[5],
                    attr("admin", admin_address.to_owned())
                );

                assert_eq!(response.messages.len(), 1);
                assert_eq!(
                    response.messages[0].msg,
                    transfer_marker_coins(
                        amount.u128(),
                        RESTRICTED_DENOM.to_owned(),
                        sender_address.to_owned(),
                        Addr::unchecked(MOCK_CONTRACT_ADDR)
                    )
                    .unwrap()
                );
            }
            Err(error) => {
                panic!("failed to reject transfer: {:?}", error)
            }
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            None,
            transfer_storage.may_load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn reject_transfer_sent_funds_returns_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let admin_address = Addr::unchecked("admin_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker: Marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(3);
        let sender_info = mock_info(admin_address.as_str(), &[coin(1, RESTRICTED_DENOM)]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let reject_transfer_msg = ExecuteMsg::RejectTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute reject transfer
        let reject_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            reject_transfer_msg.clone(),
        );

        assert_sent_funds_unsupported_error(reject_response);

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn reject_transfer_unauthorized() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let admin_address = Addr::unchecked("admin_address");
        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let test_marker =
            setup_restricted_marker_admin(RESTRICTED_DENOM.into(), admin_address.to_owned());
        deps.querier.with_markers(vec![test_marker]);

        let amount = Uint128::new(3);
        let sender_info = mock_info(sender_address.as_str(), &[]);

        let stored_transfer = Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, &stored_transfer);

        let reject_transfer_msg = ExecuteMsg::RejectTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute reject transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            reject_transfer_msg.clone(),
        );

        // verify reject transfer response
        match transfer_response {
            Ok(..) => panic!("expected error, but ok"),
            Err(error) => match error {
                ContractError::Unauthorized { .. } => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }

        let transfer_storage = get_transfer_storage_read(&deps.storage);
        assert_eq!(
            stored_transfer,
            transfer_storage.load(TRANSFER_ID.as_bytes()).unwrap()
        );
    }

    #[test]
    fn reject_transfer_unknown_transfer() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let sender_info = mock_info(sender_address.as_str(), &[]);

        let reject_transfer_msg = ExecuteMsg::RejectTransfer {
            id: TRANSFER_ID.into(),
        };

        // execute reject transfer
        let transfer_response = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            reject_transfer_msg.clone(),
        );

        assert_load_transfer_error(transfer_response);
    }

    #[test]
    fn query_transfer_by_id_test() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let sender_address = Addr::unchecked("sender_address");
        let recipient_address = Addr::unchecked("transfer_to");

        let amount = Uint128::new(3);

        let transfer = &Transfer {
            id: TRANSFER_ID.into(),
            sender: sender_address.to_owned(),
            denom: RESTRICTED_DENOM.into(),
            amount,
            recipient: recipient_address.to_owned(),
        };
        store_test_transfer(&mut deps.storage, transfer);

        let query_transfer_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTransfer {
                id: TRANSFER_ID.into(),
            },
        );

        assert_eq!(to_binary(transfer), query_transfer_response);
    }

    #[test]
    fn query_contract_info() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let query_contract_info_response =
            query(deps.as_ref(), mock_env(), QueryMsg::GetContractInfo {});

        match query_contract_info_response {
            Ok(contract_info) => {
                assert_eq!(
                    contract_info,
                    to_binary(&config_read(&deps.storage).load().unwrap()).unwrap()
                )
            }
            Err(error) => panic!("unexpected error: {:?}", error),
        }
    }

    #[test]
    fn query_version_info() {
        let mut deps = mock_dependencies(&[]);
        setup_test_base(
            &mut deps.storage,
            &State {
                name: "contract_name".into(),
            },
        );

        let result = cw2::set_contract_version(deps.as_mut().storage, CRATE_NAME, PACKAGE_VERSION);
        match result {
            Ok(..) => {}
            Err(error) => panic!("unexpected error: {:?}", error),
        }

        let query_version_info_response =
            query(deps.as_ref(), mock_env(), QueryMsg::GetVersionInfo {});

        match query_version_info_response {
            Ok(version_info) => {
                assert_eq!(
                    version_info,
                    to_binary(&cw2::get_contract_version(&deps.storage).unwrap()).unwrap()
                )
            }
            Err(error) => panic!("unexpected error: {:?}", error),
        }
    }

    fn assert_load_transfer_error(response: Result<Response<ProvenanceMsg>, ContractError>) {
        match response {
            Ok(..) => panic!("expected error, but ok"),
            Err(error) => match error {
                ContractError::LoadTransferFailed { .. } => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    fn assert_sent_funds_unsupported_error(
        response: Result<Response<ProvenanceMsg>, ContractError>,
    ) {
        match response {
            Ok(..) => panic!("expected error, but ok"),
            Err(error) => match error {
                ContractError::SentFundsUnsupported => {}
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

    fn setup_restricted_marker_admin(denom: String, admin: Addr) -> Marker {
        let marker_json = format!(
            "{{
              \"address\": \"tp1l330sxue4suxz9dhc40e2pns0ymrytf8uz4squ\",
              \"coins\": [
                {{
                  \"denom\": \"{}\",
                  \"amount\": \"1000\"
                }}
              ],
              \"account_number\": 10,
              \"sequence\": 0,
              \"permissions\": [
                {{
                  \"permissions\": [
                    \"burn\",
                    \"delete\",
                    \"deposit\",
                    \"admin\",
                    \"mint\",
                    \"withdraw\"
                  ],
                  \"address\": \"{}\"
                }}
              ],
              \"status\": \"active\",
              \"denom\": \"restricted_1\",
              \"total_supply\": \"1000\",
              \"marker_type\": \"restricted\",
              \"supply_fixed\": false
            }}",
            denom,
            admin.into_string()
        );

        return from_binary(&Binary::from(marker_json.as_bytes())).unwrap();
    }
}
