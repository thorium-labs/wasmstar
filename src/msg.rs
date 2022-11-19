use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
use cw_utils::Duration;
use nois::NoisCallback;

use crate::state::{Lottery, TicketResult};

#[cw_serde]
pub struct InstantiateMsg {
    pub treasury_fee: Coin,
    pub ticket_price: Coin,
    pub lottery_interval: Duration,
    pub nois_proxy: String,
    pub max_tickets_per_user: u32,
    pub percentage_per_match: [u8; 6],
}

#[cw_serde]
pub enum ExecuteMsg {
    BuyTicket {
        tickets: Vec<String>,
        lottery_id: u32,
    },
    ClaimLottery {
        id: u32,
    },
    ExecuteLottery {
        id: u32,
    },
    Receive {
        callback: NoisCallback,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Lottery)]
    GetLottery { id: u32 },
    #[returns(Lottery)]
    GetCurrentLottery {},
    #[returns(Vec<TicketResult>)]
    CheckWinner { addr: String, lottery_id: u32 },
}
