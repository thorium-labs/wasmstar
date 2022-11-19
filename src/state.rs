use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, CanonicalAddr, Coin, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};

#[cw_serde]
pub enum Status {
    Open,
    Pending,
    Claimable,
}

#[cw_serde]
pub struct Config {
    pub owner: CanonicalAddr,
    pub interval: Duration,
    pub treasury_fee: Coin,
    pub ticket_price: Coin,
    pub nois_proxy: Addr,
    pub percentage_per_match: [u8; 6],
    pub max_tickets_per_user: u32,
}

#[cw_serde]
pub struct Lottery {
    pub id: u32,
    pub status: Status,
    pub end_time: Expiration,
    pub winner_number: Option<String>,
    pub ticket_price: Coin,
    pub total_prize: Coin,
    pub total_tickets: u64,
    pub prize_per_match: Option<[Uint128; 6]>,
    pub winners_per_match: Option<[u32; 6]>,
}

impl Lottery {
    pub fn new(id: u32, end_time: Expiration, ticket_price: Coin) -> Self {
        return Lottery {
            id,
            status: Status::Open,
            end_time,
            winner_number: None,
            ticket_price: ticket_price.clone(),
            total_prize: coin(0, ticket_price.denom),
            total_tickets: 0u64,
            prize_per_match: None,
            winners_per_match: None,
        };
    }
}

#[cw_serde]
pub struct TicketResult {
    pub ticket_number: String,
    pub prediction: Vec<bool>,
    pub matches: u8,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const TOTAL_LOTTERIES: Item<u32> = Item::new("total_lotteries");
pub const LOTTERIES: Map<u32, Lottery> = Map::new("lotteries");
pub const WINNERS: Map<(u32, Addr), Coin> = Map::new("winners");
pub const TICKETS: Map<(u32, Addr), Vec<String>> = Map::new("tickets");
