use std::convert::Into;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw_storage_plus::Item;

pub static CONFIG_KEY: &[u8] = b"config";

pub static TRANSFER_KEY: &[u8] = b"transfer";

/// Configuration state for the restricted marker transfer contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct State {
    // The contract name
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Transfer {
    pub id: String,
    pub sender: Addr,
    pub denom: String,
    pub amount: Uint128,
    pub recipient: Addr,
}

// pub const CONFIG: Item<State> = Item::new(b"config".into());
pub const CONFIG: Item<State> = Item::new("config");

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn get_transfer_storage(storage: &mut dyn Storage) -> Bucket<Transfer> {
    bucket(storage, TRANSFER_KEY)
}

pub fn get_transfer_storage_read(storage: &dyn Storage) -> ReadonlyBucket<Transfer> {
    bucket_read(storage, TRANSFER_KEY)
}

pub fn get_all_transfers(storage: &dyn Storage) -> Vec<Transfer> {
    let stored = get_transfer_storage_read(storage);
    stored
        .range(None, None, cosmwasm_std::Order::Ascending)
        .map(|pair| pair.unwrap().1)
        .collect()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_env;
    use cw_storage_plus::Map;
    use provwasm_mocks::mock_provenance_dependencies;

    use super::*;

    #[test]
    fn migrate_test() {
        let mut deps = mock_provenance_dependencies();

        if let Err(error) = config(&mut deps.storage).save(
        &State {
                name: "contract_name".into(),
            },
        )
        {
            panic!("unexpected error: {:?}", error)
        }

        assert_eq!(
            config_read(&mut deps.storage).load().unwrap(),
            CONFIG.load(&deps.storage).unwrap()
        )
    }

    #[test]
    fn bucket_test() {
        let mut deps = mock_provenance_dependencies();

        if let Err(error) = get_ask_storage(&mut deps.storage).save(
            b"aaa",
            &State {
                name: "contract_name".into(),
            }
        ) {
            panic!("unexpected error: {:?}", error)
        }

        const ASKS_V1: Map<&[u8], State> = Map::new("asks");


        assert_eq!(
            get_ask_storage_read(&mut deps.storage).load(b"aaa").unwrap(),
            ASKS_V1.load(&deps.storage, b"aaa").unwrap()
        )
    }

    fn get_ask_storage(storage: &mut dyn Storage) -> Bucket<State> {
        bucket(storage, b"asks")
    }

    pub fn get_ask_storage_read(storage: &dyn Storage) -> ReadonlyBucket<State> {
        bucket_read(storage, b"asks")
    }
}
