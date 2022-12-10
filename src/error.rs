use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("RandomnessAlreadyRequested")]
    RandomnessAlreadyRequested,

    #[error("InvalidCoin")]
    InvalidCoin,

    #[error("MaxTicketsPerUserExceeded")]
    MaxTicketsPerUserExceeded,

    #[error("DrawIsNotOpen")]
    DrawIsNotOpen,

    #[error("DrawIsNotPending")]
    DrawIsNotPending,

    #[error("DrawIsOpen")]
    DrawIsOpen,

    #[error("InvalidTicket")]
    InvalidTicket,

    #[error("InsufficientFunds")]
    InsufficientFunds,

    #[error("InvalidRandomness")]
    InvalidRandomness,

    #[error("PrizeAlreadyClaimed")]
    PrizeAlreadyClaimed,

    #[error("DrawIsNotClaimable")]
    DrawIsNotClaimable,

    #[error("NoPrizeToClaim")]
    NoPrizeToClaim,
}
