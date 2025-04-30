use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub claim_drop_addrs: Vec<String>,
    /// Timestamp since which XKP airdrops can be delegated to boostrap auction contract
    pub from_timestamp: u64,
    /// Timestamp to which XKP airdrops can be claimed
    pub to_timestamp: u64,
    pub claim_amount: u64,
    pub denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Claim {},
    AddClaimDropAddrs {
        // Optional single address
        multiple_addrs: Vec<String>, // Optional vector of addresses
    },
    ClaimUnclaimed {
        address: String,
    },
    UpdateConfig {
        owner: Option<Addr>,
        claim_drop_addrs: Option<Vec<String>>,
        from_timestamp: Option<u64>,
        to_timestamp: Option<u64>,
        claim_amount: Option<u64>,
        airdrop_has_ended: Option<bool>,
        total_reward_pool: Option<Uint128>, // Allow owner to update total reward pool
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(ClaimDropAddressesResponse)]
    GetClaimDropAddresses {},
    // GetCount returns the current count as a json-encoded number
    #[returns(ClaimedAmountResponse)]
    GetClaimedAmount {
        address: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    /// Timestamp since which XKP airdrops can be delegated to boostrap auction contract
    pub from_timestamp: u64,
    /// Timestamp to which XKP airdrops can be claimed
    pub to_timestamp: u64,
    pub claim_amount: Uint128,
    pub airdrop_has_ended: bool,
    pub denom: String,
    pub num_active_users: u64, // Number of users who haven't fully claimed
    pub total_reward_pool: Uint128, // Total pool of tokens for distribution
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimDropAddressesResponse {
    pub claim_drop_addrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimedAmountResponse {
    pub is_included: bool,
    pub claimed_amount: Uint128,
    pub total_claimable_amount: Uint128,
    pub current_claimable_amount: Uint128,
    pub from_timestamp: u64,
    pub to_timestamp: u64,
    pub current_per_user_amount: Uint128, // Current claimable amount per user
}