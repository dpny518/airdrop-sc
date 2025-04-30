use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const STATE: Item<State> = Item::new("state");
pub const USERS: Map<&Addr, UserInfo> = Map::new("users");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct OldState {
    pub owner: Addr,
    pub claim_drop_addrs: Vec<String>,
    /// Timestamp since which XKP airdrops can be delegated to boostrap auction contract
    pub from_timestamp: u64,
    /// Timestamp to which XKP airdrops can be claimed
    pub to_timestamp: u64,
    pub claim_amount: u64,
    pub airdrop_has_ended: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub claim_drop_addrs: Vec<String>,
    /// Timestamp since which XKP airdrops can be delegated to boostrap auction contract
    pub from_timestamp: u64,
    /// Timestamp to which XKP airdrops can be claimed
    pub to_timestamp: u64,
    pub claim_amount: u64,
    pub airdrop_has_ended: bool,
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    /// Total XKP airdrop tokens claimed by the user
    pub claimed_amount: Uint128,
    pub total_claimable_amount: Uint128,
    pub current_claimable_amount: Uint128,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            claimed_amount: Uint128::zero(),
            total_claimable_amount: Uint128::zero(),
            current_claimable_amount: Uint128::zero(),
        }
    }
}
