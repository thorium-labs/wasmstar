use cosmwasm_std::{coin, Coin, Decimal, DepsMut, Env, StdError, StdResult, Uint128};
use cw_utils::Expiration;
use std::ops::{Add, Mul};

use crate::error::ContractError;
use crate::state::{Lottery, Status, TicketResult, CONFIG, LOTTERIES, TOTAL_LOTTERIES};

pub fn ensure_ticket_is_valid(ticket: &String) -> Result<(), ContractError> {
    if ticket.len() != 6 {
        return Err(ContractError::InvalidTicketLength);
    }

    Ok(())
}

/// Ensure user has sent in enought funds to cover tickets price
pub fn ensure_is_enough_funds_to_cover_tickets(
    required_funds: &Coin,
    sent_fund: &[Coin],
) -> Result<(), ContractError> {
    let fund = sent_fund
        .iter()
        .find(|c| c.denom == required_funds.denom)
        .ok_or_else(|| {
            ContractError::Std(StdError::GenericErr {
                msg: format!("Expected denom fee: {}", required_funds.denom),
            })
        })?;

    if fund.amount < required_funds.amount {
        return Err(ContractError::InvalidAmount);
    }

    Ok(())
}

pub fn create_next_lottery(deps: DepsMut, env: Env) -> StdResult<()> {
    let lottery_id = TOTAL_LOTTERIES.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let end_time = Expiration::AtTime(env.block.time)
        .add(config.interval)
        .expect("error defining end_time");

    let lottery = Lottery {
        id: 1,
        status: Status::Open,
        end_time,
        winner_number: None,
        ticket_price: config.ticket_price.clone(),
        total_prize: coin(0, config.ticket_price.denom),
        total_tickets: 0u64,
        prize_per_match: None,
        winners_per_match: None,
    };

    println!("Creating new lottery: {:?}", lottery.clone());

    LOTTERIES.save(deps.storage, 1, &lottery.clone())?;

    Ok(())
}

pub fn calculate_prize_distribution(
    total_amount: Uint128,
    percent_per_matches: [u8; 6],
) -> [Uint128; 6] {
    percent_per_matches.map(|p| -> Uint128 {
        let percent = Decimal::percent(p.into());
        total_amount.mul(percent)
    })
}

pub fn calculate_tickets_prize(
    tickets: Vec<TicketResult>,
    prize_per_match: [Uint128; 6],
    winners_per_match: [u32; 6],
    denom: String,
) -> Coin {
    let mut prize = Uint128::zero();

    for ticket in tickets {
        if ticket.matches > 0 {
            let index = ticket.matches as usize - 1;
            let ticket_prize = prize_per_match[index]
                .checked_div(winners_per_match[index].into())
                .expect("error calculating ticket prize");
            prize += ticket_prize;
        }
    }

    coin(prize.u128(), denom)
}

pub fn calculate_winner_per_match(
    tickets: impl Iterator<Item = String>,
    winning_ticket: String,
) -> [u32; 6] {
    tickets
        .map(|t| -> u32 {
            t.chars()
                .zip(winning_ticket.chars())
                .filter_map(|(a, b)| (a == b).then_some(true))
                .count() as u32
        })
        .fold([0; 6], |mut acc, n| {
            if n > 0 {
                acc[n as usize - 1] += 1;
            }
            acc
        })
}

pub fn check_tickets(tickets: Vec<String>, winning_ticket: String) -> Vec<TicketResult> {
    tickets
        .iter()
        .map(|t| -> TicketResult {
            let prediction = t.chars().zip(winning_ticket.chars()).map(|(a, b)| a == b);
            let matches = prediction.clone().filter(|&x| x).count() as u8;
            TicketResult {
                ticket_number: t.clone(),
                prediction: prediction.collect(),
                matches,
            }
        })
        .collect()
}
