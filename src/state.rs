use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, Bucket, ReadonlyBucket,
};
use cw_storage_plus::Item;

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

pub const CONFIG: Item<State> = Item::new("config");

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
