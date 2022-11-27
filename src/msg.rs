use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
use cw_utils::Duration;
use nois::NoisCallback;

use crate::state::{Config, Draw, TicketResult};

#[cw_serde]
pub struct InstantiateMsg {
    pub treasury_fee: u8,
    pub ticket_price: Coin,
    pub draw_interval: Duration,
    pub nois_proxy: String,
    pub max_tickets_per_user: u32,
    pub percentage_per_match: [u8; 6],
}

#[cw_serde]
pub enum ExecuteMsg {
    BuyTicket { tickets: Vec<String>, draw_id: u64 },
    ClaimPrize { draw_id: u64 },
    ExecuteDraw { id: u64 },
    Receive { callback: NoisCallback },
    UpdateConfig { new_config: UpdateConfigMsg },
    // Staking Executions
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
    pub ticket_price: Option<Coin>,
    pub interval: Option<Duration>,
    pub nois_proxy: Option<String>,
    pub max_tickets_per_user: Option<u32>,
    pub percentage_per_match: Option<[u8; 6]>,
}
