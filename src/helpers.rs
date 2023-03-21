use cosmwasm_std::{coin, Addr, Coin, DepsMut, Env, StdResult, Uint128};
use cw_utils::{Duration, Expiration};
use std::ops::Add;

use crate::error::ContractError;
use crate::state::{Draw, TicketResult, CONFIG, DRAWS, DRAWS_INDEX};

pub fn ensure_ticket_is_valid(ticket: &String) -> Result<(), ContractError> {
    if ticket.len().ne(&6) {
        return Err(ContractError::InvalidTicket);
    }

    let in_range = 0..=999999;
    let ticket_number = ticket
        .parse::<u64>()
        .map_err(|_| ContractError::InvalidTicket)?;

    if !in_range.contains(&ticket_number) {
        return Err(ContractError::InvalidTicket);
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
        .ok_or(ContractError::InvalidCoin)?;

    if fund.amount < required_funds.amount {
        return Err(ContractError::InsufficientFunds);
    }

    Ok(())
}

pub fn create_next_draw(deps: DepsMut, env: &Env, inital_prize: Uint128) -> StdResult<()> {
    let id = DRAWS_INDEX.update(deps.storage, |id: u64| -> StdResult<u64> { Ok(id.add(1)) })?;
    let config = CONFIG.load(deps.storage)?;

    let end_time = match config.interval {
        Duration::Height(_) => Expiration::AtHeight(env.block.height).add(config.interval),
        Duration::Time(_) => Expiration::AtTime(env.block.time).add(config.interval),
    }?;

    let prize_per_match = Some(calculate_prize_distribution(
        inital_prize,
        config.percentage_per_match,
    ));

    DRAWS.save(
        deps.storage,
        id,
        &Draw::new(
            id,
            end_time,
            config.ticket_price,
            inital_prize,
            prize_per_match,
        ),
    )?;

    Ok(())
}

pub fn calculate_prize_distribution(
    total_amount: Uint128,
    percent_per_matches: [u8; 6],
) -> [Uint128; 6] {
    percent_per_matches.map(|p| total_amount.multiply_ratio(p, Uint128::from(100u128)))
}

pub fn calculate_tickets_prize(
    tickets: Vec<TicketResult>,
    prize_per_match: [Uint128; 6],
    winners_per_match: [u64; 6],
    denom: String,
) -> Coin {
    let prize = tickets.iter().fold(Uint128::zero(), |mut acc, t| {
        if t.matches > 0 {
            let index = t.matches as usize - 1;
            let ticket_prize = prize_per_match[index]
                .checked_div(winners_per_match[index].into())
                .expect("error calculating ticket prize");
            acc = acc
                .checked_add(ticket_prize)
                .expect("error calculating ticket prize")
        }
        acc
    });

    coin(prize.u128(), denom)
}

pub fn calculate_winner_per_match(
    tickets: Vec<(Addr, Vec<String>)>,
    winning_ticket: String,
) -> [u64; 6] {
    tickets
        .iter()
        .fold([0, 0, 0, 0, 0, 0], |mut acc, (_, utickets)| {
            utickets.iter().for_each(|t| {
                let matches = calculate_matches(t, &winning_ticket);
                if matches > 0 {
                    acc[matches as usize - 1] += 1;
                }
            });
            acc
        })
}

pub fn calculate_matches(winning_ticket: &str, ticket: &str) -> u8 {
    let mut matches = 0;

    for (winning_number, number) in winning_ticket.chars().zip(ticket.chars()) {
        if winning_number == number {
            matches += 1;
        } else {
            break;
        }
    }

    matches
}

pub fn check_tickets(tickets: Vec<String>, winning_ticket: String) -> Vec<TicketResult> {
    tickets
        .iter()
        .map(|t| -> TicketResult {
            let matches = calculate_matches(t, &winning_ticket);
            TicketResult {
                ticket_number: t.clone(),
                matches,
            }
        })
        .collect()
}

pub fn build_expiration_time(env: &Env, duration: Duration) -> StdResult<Expiration> {
    match duration {
        Duration::Height(_) => Expiration::AtHeight(env.block.height).add(duration),
        Duration::Time(_) => Expiration::AtTime(env.block.time).add(duration),
    }
}
