use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("InvalidProxyAddr")]
    InvalidProxyAddr,

    #[error("InvalidPriceTicket")]
    InvalidPriceTicket,

    #[error("InvalidDenom")]
    InvalidDenom,

    #[error("MaxTicketsPerUserExceeded")]
    MaxTicketsPerUserExceeded,

    #[error("LotteryIsNotOpen")]
    LotteryIsNotOpen,

    #[error("LotteryStillOpen")]
    LotteryStillOpen,

    #[error("InvalidTicket")]
    InvalidTicket,

    #[error("Invalid Amount")]
    InvalidAmount,

    #[error("InvalidRandomness")]
    InvalidRandomness,

    #[error("AlreadyClaimed")]
    AlreadyClaimed,

    #[error("LotteryIsNotClaimable")]
    LotteryIsNotClaimable,
}
