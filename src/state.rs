use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::Into;

use cosmwasm_std::{Addr, Storage, Uint128};
use cw_storage_plus::{Item, Map};

pub const STORAGE_TRANSFER_KEY: &str = "transfer";

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

pub const TRANSFER_STORAGE: Map<&[u8], Transfer> = Map::new(STORAGE_TRANSFER_KEY);

pub fn get_all_transfers(storage: &dyn Storage) -> Vec<Transfer> {
    TRANSFER_STORAGE
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|pair| pair.unwrap().1)
        .collect()
}
