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
    pub treasury_fee: u8,
    pub ticket_price: Coin,
    pub nois_proxy: Addr,
    pub percentage_per_match: [u8; 6],
    pub max_tickets_per_user: u32,
}

#[cw_serde]
pub struct Draw {
    pub id: u64,
    pub status: Status,
    pub end_time: Expiration,
    pub winner_number: Option<String>,
    pub ticket_price: Coin,
    pub total_prize: Coin,
    pub total_tickets: u64,
    pub prize_per_match: Option<[Uint128; 6]>,
    pub winners_per_match: Option<[u64; 6]>,
}

impl Draw {
    pub fn new(id: u64, end_time: Expiration, ticket_price: Coin) -> Self {
        return Draw {
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
    pub matches: u8,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const DRAWS_INDEX: Item<u64> = Item::new("draws_index");
pub const DRAWS: Map<u64, Draw> = Map::new("draws");
pub const WINNERS: Map<(u64, Addr), Coin> = Map::new("winners");
pub const TICKETS: Map<(u64, Addr), Vec<String>> = Map::new("tickets");
