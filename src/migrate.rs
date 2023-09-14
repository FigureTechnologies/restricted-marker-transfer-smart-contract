use cosmwasm_std::{entry_point, DepsMut, Env, Response};
use cw2::set_contract_version;
use cw_storage_plus::Item;
use semver::{Version, VersionReq};

use crate::contract::{CRATE_NAME, PACKAGE_VERSION};
use crate::error::ContractError;
use crate::msg::MigrateMsg;
use crate::state::{State, CONFIG};
use crate::ContractError::{InvalidContractType, UnsupportedUpgrade};

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let stored_contract_version = cw2::get_contract_version(deps.storage)?;

    // ensure we are migrating from an allowed contract
    if stored_contract_version.contract != CRATE_NAME {
        return Err(InvalidContractType);
    }

    let new_version = PACKAGE_VERSION.parse::<Version>().unwrap();
    let current_version = stored_contract_version.version.parse::<Version>().unwrap();
    if current_version > new_version {
        return Err(UnsupportedUpgrade {
            source_version: stored_contract_version.version,
            target_version: new_version.to_string(),
        });
    }

    let config_migration_req = VersionReq::parse("<0.3.0").unwrap();

    if config_migration_req.matches(&current_version) {
        if CONFIG.may_load(deps.storage)?.is_none() {
            // when migrating from cosmwasm-storage::Singleton to Item, cosmwasm_std::storage_keys::to_length_prefixed
            // was used for the key. Hardcoding this value to copy the legacy storage
            const LEGACY_CONFIG: Item<State> = Item::new("\0\u{6}config");
            let state = LEGACY_CONFIG.load(deps.storage).unwrap();
            CONFIG.save(deps.storage, &state)?;
            LEGACY_CONFIG.remove(deps.storage)
        }
    }

    set_contract_version(deps.storage, CRATE_NAME, PACKAGE_VERSION)?;
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_env;
    use provwasm_mocks::mock_provenance_dependencies;

    use super::*;

    #[test]
    fn migrate_test() {
        let mut deps = mock_provenance_dependencies();

        let result = set_contract_version(deps.as_mut().storage, CRATE_NAME, "0.3.0");

        match result {
            Ok(..) => {}
            Err(error) => panic!("unexpected error: {:?}", error),
        }

        let migrate_response = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

        // verify migrate response
        match migrate_response {
            Ok(..) => {
                let version_info = cw2::get_contract_version(&deps.storage).unwrap();

                assert_eq!(PACKAGE_VERSION, version_info.version);
                assert_eq!(CRATE_NAME, version_info.contract);
            }
            error => panic!("failed to initialize: {:?}", error),
        }
    }

    #[test]
    fn test_migrate_legacy_config() {
        let mut deps = mock_provenance_dependencies();

        let contract_info = State { name: "rmt".into() };

        // store legacy config state.
        const LEGACY_CONFIG: Item<State> = Item::new("\0\u{6}config");
        LEGACY_CONFIG
            .save(&mut deps.storage, &contract_info)
            .unwrap();

        set_contract_version(deps.as_mut().storage, CRATE_NAME, "0.2.0").unwrap();

        let migrate_response = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

        match migrate_response {
            Ok(..) => {}
            error => panic!("failed to initialize: {:?}", error),
        }

        assert_eq!(contract_info, CONFIG.load(&deps.storage).unwrap())
    }

    #[test]
    fn test_migrate_invalid_contract_type() {
        let mut deps = mock_provenance_dependencies();

        set_contract_version(deps.as_mut().storage, "other_name", "0.3.0").unwrap();

        let migrate_response = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

        match migrate_response {
            Ok(..) => panic!("migration should fail when the contract type is changing"),
            Err(error) => match error {
                InvalidContractType => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }

    #[test]
    fn test_migrate_invalid_version() {
        let mut deps = mock_provenance_dependencies();
        let current_version: String = "999.0.0".into();
        let new_version: String = String::from(PACKAGE_VERSION);

        set_contract_version(deps.as_mut().storage, CRATE_NAME, current_version).unwrap();

        let migrate_response = migrate(deps.as_mut(), mock_env(), MigrateMsg {});

        match migrate_response {
            Ok(..) => panic!("migration should fail when the version is decreasing"),
            Err(error) => match error {
                UnsupportedUpgrade {
                    source_version: current_version,
                    target_version: new_version,
                } => {}
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }
}
