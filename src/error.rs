use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("User has already fully claimed their rewards")]
    FullyClaimed {},

    #[error("Insufficient contract balance: required {required}, available {available}")]
    InsufficientBalance { required: Uint128, available: Uint128 },

    #[error("Invalid total reward pool: must be at least {minimum}")]
    InvalidRewardPool { minimum: Uint128 },
}