use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
use cw_utils::Duration;
use nois::NoisCallback;

use crate::state::{Config, Draw, TicketResult};

#[cw_serde]
pub struct InstantiateMsg {
    pub treasury_fee: u8,
    pub request_timeout: Duration,
    pub ticket_price: Coin,
    pub draw_interval: Duration,
    pub nois_proxy: String,
    pub max_tickets_per_user: u32,
    pub percentage_per_match: [u8; 6],
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive { callback: NoisCallback },
    RequestRandomness { draw_id: u64 },
    BuyTickets { tickets: Vec<String>, draw_id: u64 },
    Raffle { draw_id: u64 },
    ClaimPrize { draw_id: u64 },
    UpdateConfig { new_config: UpdateConfigMsg },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Option<Draw>)]
    GetDraw { id: u64 },
    #[returns(Draw)]
    GetCurrentDraw {},
    #[returns(Vec<TicketResult>)]
    CheckWinner { addr: String, draw_id: u64 },
    #[returns(Vec<String>)]
    GetTickets { addr: String, draw_id: u64 },
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub treasury_fee: Option<u8>,
    pub owner: Option<String>,
    pub ticket_price: Option<Coin>,
    pub interval: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub nois_proxy: Option<String>,
    pub max_tickets_per_user: Option<u32>,
    pub percentage_per_match: Option<[u8; 6]>,
}
