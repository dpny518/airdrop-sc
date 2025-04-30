use crate::state::{OldState, USERS};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Item;

use crate::error::ContractError;
use crate::msg::{
    ClaimedAmountResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{State, UserInfo, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:claim-drop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let total_claim: Uint128 = msg.claim_amount.into();
    let addrs = msg.claim_drop_addrs.clone();
    for addr in addrs {
        let validated_addr = deps.api.addr_validate(&addr)?;
        let mut user_info = USERS
            .load(deps.storage, &validated_addr)
            .unwrap_or_default();
        user_info.total_claimable_amount = total_claim;
        USERS.save(deps.storage, &validated_addr, &user_info)?;
    }

    let state = State {
        owner: msg.owner,
        claim_drop_addrs: msg.claim_drop_addrs.clone(),
        from_timestamp: msg.from_timestamp,
        to_timestamp: msg.to_timestamp,
        claim_amount: msg.claim_amount,
        airdrop_has_ended: false,
        denom: msg.denom,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("sender", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, StdError> {
    match msg {
        ExecuteMsg::Claim {} => execute::claim(deps, env, info),
        ExecuteMsg::AddClaimDropAddrs { multiple_addrs } => {
            execute::add_claim_drop_addrs(deps, env, info, multiple_addrs)
        }
        ExecuteMsg::ClaimUnclaimed { address } => {
            let validated_addr = deps.api.addr_validate(&address)?;
            execute::claim_unclaimed(deps, env, info, validated_addr)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            claim_drop_addrs,
            from_timestamp,
            to_timestamp,
            claim_amount,
            airdrop_has_ended,
        } => execute::update_config(
            deps,
            info,
            owner,
            claim_drop_addrs,
            from_timestamp,
            to_timestamp,
            claim_amount,
            airdrop_has_ended,
        ),
    }
}

pub mod execute {
    use super::*;
    use crate::state::USERS;
    use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, StdError, Uint128};

    pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, StdError> {
        let addr = info.sender;
        let state = STATE.load(deps.storage)?;
        let mut claimable_amount_for_user = Uint128::zero();
        let mut user_info = USERS.load(deps.storage, &addr).unwrap_or_default();

        if state.airdrop_has_ended {
            return Err(StdError::generic_err("Airdrop is closed"));
        }

        if env.block.time.seconds() < state.from_timestamp {
            return Err(StdError::generic_err("Claim not allowed"));
        }

        if !state.claim_drop_addrs.contains(&addr.to_string()) {
            return Err(StdError::generic_err("No claim for address"));
        }

        let claimable_amount_for_user =
            get_claimable_amount_for_user(env, state.clone(), user_info.clone())?;

        if claimable_amount_for_user.is_zero() {
            return Err(StdError::generic_err("Nothing to claim"));
        }
        user_info.claimed_amount = user_info.claimed_amount + claimable_amount_for_user;
        if user_info.claimed_amount > state.claim_amount.into() {
            return Err(StdError::generic_err("Claimed more than  allowed"));
        }

        USERS.save(deps.storage, &addr, &user_info)?;

        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: addr.into_string(),
            amount: vec![Coin {
                amount: claimable_amount_for_user,
                denom: state.denom,
            }],
        });

        Ok(Response::new()
            .add_message(send_msg)
            .add_attribute("method", "send_tokens")
            .add_attribute("amount", state.claim_amount.to_string()))
    }

    pub fn add_claim_drop_addrs(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        multiple_addrs: Vec<String>,
    ) -> Result<Response, StdError> {
        let mut state = STATE.load(deps.storage)?;

        if info.sender != state.owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        let addrs = multiple_addrs;
        for addr in addrs {
            let validated_addr = deps.api.addr_validate(&addr)?;
            if !state.claim_drop_addrs.contains(&validated_addr.to_string()) {
                let mut user_info = USERS
                    .load(deps.storage, &validated_addr)
                    .unwrap_or_default();
                user_info.total_claimable_amount = state.claim_amount.into();
                USERS.save(deps.storage, &validated_addr, &user_info)?;
                state.claim_drop_addrs.push(validated_addr.to_string());
            }
        }

        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_attribute("method", "add_claim_drop_addrs"))
    }

    pub fn claim_unclaimed(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        addr: Addr,
    ) -> Result<Response, StdError> {
        let state = STATE.load(deps.storage)?;

        if info.sender != state.owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        let contract_address = env.contract.address;
        let balance = deps.querier.query_balance(contract_address, &state.denom)?;
        if balance.amount.is_zero() {
            return Err(StdError::generic_err("Insufficient contract balance"));
        }

        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: addr.into_string(),
            amount: vec![Coin {
                amount: balance.amount,
                denom: state.denom,
            }],
        });

        let mut state = STATE.load(deps.storage)?;
        state.airdrop_has_ended = true;
        STATE.save(deps.storage, &state)?;

        Ok(Response::new()
            .add_message(send_msg)
            .add_attribute("method", "claim_unclaimed")
            .add_attribute("recipient", info.sender)
            .add_attribute("amount", balance.amount.to_string())
            .add_attribute("denom", state.denom))
    }

    pub fn update_config(
        deps: DepsMut,
        info: MessageInfo,
        owner: Option<Addr>,
        claim_drop_addrs: Option<Vec<String>>,
        from_timestamp: Option<u64>,
        to_timestamp: Option<u64>,
        claim_amount: Option<u64>,
        airdrop_has_ended: Option<bool>,
    ) -> Result<Response, StdError> {
        let mut state = STATE.load(deps.storage)?;

        if info.sender != state.owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        if let Some(new_owner) = owner {
            state.owner = new_owner;
        }
        if let Some(new_addrs) = claim_drop_addrs {
            state.claim_drop_addrs = new_addrs;
        }
        if let Some(new_from) = from_timestamp {
            state.from_timestamp = new_from;
        }
        if let Some(new_to) = to_timestamp {
            state.to_timestamp = new_to;
        }
        if let Some(new_amount) = claim_amount {
            state.claim_amount = new_amount;
        }
        if let Some(new_ended) = airdrop_has_ended {
            state.airdrop_has_ended = new_ended;
        }

        STATE.save(deps.storage, &state)?;

        Ok(Response::new()
            .add_attribute("method", "update_config")
            .add_attribute("updated_by", info.sender))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => {
            to_json_binary(&query::config(deps)?)
        },
        QueryMsg::GetClaimDropAddresses {} => {
            to_json_binary(&query::get_claim_drop_addresses(deps)?)
        },
        QueryMsg::GetClaimedAmount { address } => {
            to_json_binary(&query::get_user_claimed_amount(deps, _env, address)?)
        }
    }
}

fn get_claimable_amount_for_user(
    env: Env,
    state: State,
    user_info: UserInfo,
) -> StdResult<Uint128> {
    let mut claimable_amount_for_user = Uint128::zero();
    if env.block.time.seconds() >= state.to_timestamp {
        let claim_amount_u: Uint128 = state.claim_amount.into();
        claimable_amount_for_user = claim_amount_u - user_info.claimed_amount;
    } else {
        let vesting_period = state.to_timestamp - state.from_timestamp;
        if vesting_period == 0 {
            return Err(StdError::generic_err("Invalid vesting period"));
        }
        let current_time: u64 = env.block.time.seconds();
        let period_left = state.to_timestamp - current_time;
        let elapsed_time = vesting_period - period_left; // Time elapsed since start

        // Use fixed-point arithmetic with a scale of 1,000,000 (6 decimal places)
        let scale = Uint128::from(1_000_000u128);
        let period_finished_scaled = Uint128::from(elapsed_time)
            .checked_mul(scale)?
            .checked_div(Uint128::from(vesting_period))?;
        let multi: Uint128 = state.claim_amount.into();
        let claimable_amount = (multi * period_finished_scaled) / scale;
        claimable_amount_for_user = claimable_amount - user_info.claimed_amount;
    }
    Ok(claimable_amount_for_user)
}

pub mod query {
    use super::*;
    use crate::{msg::ClaimDropAddressesResponse, state::USERS};
    use cosmwasm_std::{Addr, Uint128};

    pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
        let state = STATE.load(deps.storage)?;

        Ok(ConfigResponse {
            owner: state.owner,
            denom: state.denom,
            from_timestamp: state.from_timestamp,
            to_timestamp: state.to_timestamp,
            claim_amount: state.claim_amount.into(),
            airdrop_has_ended: state.airdrop_has_ended,
        })
    }

    pub fn get_claim_drop_addresses(deps: Deps) -> StdResult<ClaimDropAddressesResponse> {
        let state = STATE.load(deps.storage)?;

        Ok(ClaimDropAddressesResponse {
            claim_drop_addrs: state.claim_drop_addrs,
        })
    }


    pub fn get_user_claimed_amount(
        deps: Deps,
        env: Env,
        address: Addr,
    ) -> StdResult<ClaimedAmountResponse> {
        let state = STATE.load(deps.storage)?;
        let user_info = USERS.load(deps.storage, &address).unwrap_or_default();

        let mut claimable_amount_for_user = Uint128::zero();
        let mut is_included = false;

        if user_info.total_claimable_amount != Uint128::zero() {
            is_included = true;
            claimable_amount_for_user =
                get_claimable_amount_for_user(env, state.clone(), user_info.clone())?;
        }

        Ok(ClaimedAmountResponse {
            is_included: is_included,
            claimed_amount: user_info.claimed_amount,
            total_claimable_amount: user_info.total_claimable_amount,
            current_claimable_amount: claimable_amount_for_user,
            from_timestamp: state.from_timestamp,
            to_timestamp: state.to_timestamp,
        })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let old_state: OldState = Item::new("state").load(deps.storage)?;
    let new_state = State {
        claim_drop_addrs: old_state.claim_drop_addrs,
        owner: old_state.owner,
        from_timestamp: old_state.from_timestamp,
        to_timestamp: old_state.to_timestamp,
        claim_amount: old_state.claim_amount,
        airdrop_has_ended: old_state.airdrop_has_ended,
        denom: "ukopi".to_string(), // Set the new property
    };
    STATE.save(deps.storage, &new_state)?;
    Ok(Response::new().add_attribute("method", "migrate"))
}
