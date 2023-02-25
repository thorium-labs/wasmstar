use std::num::ParseIntError;

use cosmwasm_std::{DivideByZeroError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    DividedByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),

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
