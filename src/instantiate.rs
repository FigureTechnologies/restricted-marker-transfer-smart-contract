use crate::contract::{CRATE_NAME, PACKAGE_VERSION};
use crate::error::contract_err;
use crate::msg::{InstantiateMsg, Validate};
use crate::state::{State, CONFIG};
use crate::ContractError;
use cosmwasm_std::{attr, entry_point, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

/// Create the initial configuration state
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;
    // Validate params
    if !info.funds.is_empty() {
        return Err(contract_err("no funds should be sent during instantiate"));
    }
    // Create and store config state.
    let contract_info = State { name: msg.name };
    CONFIG.save(deps.storage, &contract_info)?;

    set_contract_version(deps.storage, CRATE_NAME, PACKAGE_VERSION)?;

    // build response
    Ok(Response::new().add_attributes(vec![
        attr("contract_info", format!("{:?}", CONFIG.load(deps.storage)?)),
        attr("action", "init"),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use provwasm_mocks::mock_provenance_dependencies;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_provenance_dependencies();
        let info = mock_info("contract_admin", &[]);

        let contract_name = "please transfer me";

        let init_msg = InstantiateMsg {
            name: contract_name.into(),
        };

        let init_response = instantiate(deps.as_mut(), mock_env(), info, init_msg.clone());

        // verify initialize response
        match init_response {
            Ok(init_response) => {
                assert_eq!(init_response.messages.len(), 0);

                assert_eq!(init_response.attributes.len(), 2);

                let expected_state = State {
                    name: contract_name.into(),
                };

                assert_eq!(
                    init_response.attributes[0],
                    attr("contract_info", format!("{:?}", expected_state))
                );
                assert_eq!(init_response.attributes[1], attr("action", "init"));

                let version_info = cw2::get_contract_version(&deps.storage).unwrap();

                assert_eq!(PACKAGE_VERSION, version_info.version);
                assert_eq!(CRATE_NAME, version_info.contract);
            }
            error => panic!("failed to initialize: {:?}", error),
        }
    }
}
