#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coin, ensure_eq, to_binary, wasm_execute, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128,
};
use nois::{ints_in_range, NoisCallback, ProxyExecuteMsg};
use std::ops::{Add, Mul, Sub};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::{
    calculate_prize_distribution, calculate_tickets_prize, calculate_winner_per_match,
    check_tickets, create_next_lottery, ensure_is_enough_funds_to_cover_tickets,
    ensure_ticket_is_valid,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfigMsg};
use crate::state::{
    Config, Lottery, Status, TicketResult, CONFIG, LOTTERIES, TICKETS, TOTAL_LOTTERIES, WINNERS,
};

const CONTRACT_NAME: &str = "crates.io:super-star";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let nois_proxy_addr = deps
        .api
        .addr_validate(&msg.nois_proxy)
        .map_err(|_| ContractError::InvalidProxyAddr)?;

    if msg.ticket_price.amount.is_zero() {
        return Err(ContractError::InvalidPriceTicket);
    }

    let config = Config {
        owner: deps.api.addr_canonicalize(&info.sender.as_str())?,
        interval: msg.lottery_interval,
        ticket_price: msg.ticket_price,
        nois_proxy: nois_proxy_addr,
        treasury_fee: msg.treasury_fee,
        percentage_per_match: msg.percentage_per_match,
        max_tickets_per_user: msg.max_tickets_per_user,
    };

    CONFIG.save(deps.storage, &config)?;
    TOTAL_LOTTERIES.save(deps.storage, &0)?;

    create_next_lottery(deps, env)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BuyTicket {
            tickets,
            lottery_id,
        } => buy_tickets(deps, env, info, tickets, lottery_id),
        ExecuteMsg::ClaimLottery { id } => claim_lottery(deps, info, id),
        ExecuteMsg::ExecuteLottery { id } => execute_lottery(deps, env, info, id),
        ExecuteMsg::Receive { callback } => random_callback(deps, info, callback),
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, info, new_config),
    }
}

pub fn buy_tickets(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tickets: Vec<String>,
    lottery_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_lottery = TOTAL_LOTTERIES.load(deps.storage)?;
    let mut lottery = LOTTERIES.load(deps.storage, current_lottery)?;

    if lottery.status != Status::Open || lottery.end_time.is_expired(&env.block) {
        return Err(ContractError::LotteryIsNotOpen);
    }

    let mut tickets_bought = TICKETS
        .may_load(deps.storage, (lottery_id, info.sender.clone()))?
        .unwrap_or_default();

    let n_tickets = tickets.len() as u32;

    if n_tickets.add(tickets_bought.len() as u32) > config.max_tickets_per_user {
        return Err(ContractError::MaxTicketsPerUserExceeded);
    }

    tickets.iter().try_for_each(ensure_ticket_is_valid)?;

    let required_funds = coin(
        config.ticket_price.amount.u128().mul(u128::from(n_tickets)),
        config.ticket_price.denom,
    );

    ensure_is_enough_funds_to_cover_tickets(&required_funds, &info.funds)?;

    tickets_bought.extend(tickets);

    TICKETS.save(
        deps.storage,
        (current_lottery, info.sender),
        &tickets_bought,
    )?;

    lottery.total_tickets = lottery.total_tickets.add(u64::from(n_tickets));
    lottery.total_prize.amount += required_funds.amount;

    lottery.prize_per_match = Some(calculate_prize_distribution(
        lottery.total_prize.amount.clone(),
        config.percentage_per_match.clone(),
    ));

    LOTTERIES.save(deps.storage, current_lottery, &lottery)?;

    Ok(Response::default())
}

pub fn claim_lottery(
    deps: DepsMut,
    info: MessageInfo,
    lottery_id: u64,
) -> Result<Response, ContractError> {
    // TODO: Remove winners and use indexer
    if WINNERS
        .may_load(deps.storage, (lottery_id, info.sender.clone()))?
        .is_some()
    {
        return Err(ContractError::AlreadyClaimed);
    }

    let lottery = LOTTERIES.load(deps.storage, lottery_id)?;

    if lottery.status != Status::Claimable {
        return Err(ContractError::LotteryIsNotClaimable);
    }

    let tickets = TICKETS.load(deps.storage, (lottery_id, info.sender.clone()))?;

    let t_result = check_tickets(
        tickets,
        lottery
            .winner_number
            .ok_or(ContractError::InvalidRandomness)?,
    );

    let prize = calculate_tickets_prize(
        t_result,
        lottery.prize_per_match.expect("prize per match is not set"),
        lottery
            .winners_per_match
            .expect("winners per match is not set"),
        lottery.ticket_price.clone().denom,
    );

    WINNERS.save(
        deps.storage,
        (lottery_id, info.sender.clone()),
        &lottery.ticket_price,
    )?;

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![prize.clone()],
        }))
        .add_attribute("action", "claim_lottery")
        .add_attribute("lottery_id", lottery_id.to_string())
        .add_attribute("winner", info.sender.to_string())
        .add_attribute("prize", prize.to_string()))
}

pub fn execute_lottery(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lottery_id: u64,
) -> Result<Response, ContractError> {
    let mut lottery = LOTTERIES.load(deps.storage, lottery_id.clone())?;

    if !lottery.end_time.is_expired(&env.block) {
        return Err(ContractError::LotteryStillOpen);
    }

    let config = CONFIG.load(deps.storage)?;

    let msg = wasm_execute(
        config.nois_proxy,
        &ProxyExecuteMsg::GetNextRandomness {
            job_id: lottery_id.to_string(),
        },
        info.funds,
    )?;

    lottery.status = Status::Pending;

    LOTTERIES.save(deps.storage, lottery_id, &lottery)?;
    create_next_lottery(deps, env)?;

    Ok(Response::new().add_message(msg))
}

pub fn random_callback(
    deps: DepsMut,
    info: MessageInfo,
    callback: NoisCallback,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(info.sender, config.nois_proxy, ContractError::Unauthorized);

    let randomness: [u8; 32] = callback
        .randomness
        .to_array()
        .map_err(|_| ContractError::InvalidRandomness)?;

    let result: [u8; 6] = ints_in_range(randomness, 0..=9);

    let winner_number = result
        .into_iter()
        .fold(String::new(), |acc, x| acc + &x.to_string());

    let lottery_id = callback
        .job_id
        .parse::<u64>()
        .expect("error parsing job_id");

    let purchases = TICKETS
        .prefix(lottery_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(Addr, Vec<String>)>>>()?;

    let mut lottery = LOTTERIES.load(deps.storage, lottery_id)?;

    let winners_per_match = calculate_winner_per_match(purchases, winner_number.clone());

    lottery.winner_number = Some(winner_number.clone());
    lottery.status = Status::Claimable;
    lottery.winners_per_match = Some(winners_per_match.clone());

    LOTTERIES.save(deps.storage, lottery_id, &lottery)?;

    let config = CONFIG.load(deps.storage)?;
    let mut current_lottery = LOTTERIES.load(deps.storage, lottery_id + 1)?;

    let accumulative_pot = lottery
        .prize_per_match
        .unwrap()
        .iter()
        .fold(Uint128::zero(), |acc, x| acc.add(x.clone()));

    let treasury_reward = accumulative_pot.mul(Decimal::percent(config.treasury_fee.into()));

    current_lottery.total_prize.amount = current_lottery
        .total_prize
        .amount
        .add(accumulative_pot.sub(treasury_reward));

    Ok(Response::default())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    config: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    let mut current_config = CONFIG.load(deps.storage)?;

    if info.sender != deps.api.addr_humanize(&current_config.owner)? {
        return Err(ContractError::Unauthorized);
    }

    if let Some(new_nois_proxy) = config.nois_proxy {
        current_config.nois_proxy = deps.api.addr_validate(new_nois_proxy.as_str())?;
    }

    if let Some(new_treasury_fee) = config.treasury_fee {
        current_config.treasury_fee = new_treasury_fee;
    }

    if let Some(new_percentage_per_match) = config.percentage_per_match {
        current_config.percentage_per_match = new_percentage_per_match;
    }

    if let Some(new_ticket_price) = config.ticket_price {
        current_config.ticket_price = new_ticket_price;
    }

    if let Some(new_max_tickets_per_user) = config.max_tickets_per_user {
        current_config.max_tickets_per_user = new_max_tickets_per_user;
    }

    CONFIG.save(deps.storage, &current_config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCurrentLottery {} => to_binary(&get_current_lottery(deps)?),
        QueryMsg::GetLottery { id } => to_binary(&get_lottery(deps, id)?),
        QueryMsg::CheckWinner { addr, lottery_id } => {
            to_binary(&check_winner(deps, addr, lottery_id)?)
        }
        QueryMsg::GetTickets { addr, lottery_id } => {
            to_binary(&get_tickets(deps, lottery_id, addr)?)
        }
        QueryMsg::GetConfig {} => to_binary(&get_config(deps)?),
    }
}

pub fn get_current_lottery(deps: Deps) -> StdResult<Lottery> {
    Ok(LOTTERIES.load(deps.storage, TOTAL_LOTTERIES.load(deps.storage)?)?)
}

pub fn get_lottery(deps: Deps, lottery_id: u64) -> StdResult<Option<Lottery>> {
    Ok(LOTTERIES.may_load(deps.storage, lottery_id)?)
}

pub fn get_tickets(deps: Deps, lottery_id: u64, addr: String) -> StdResult<Vec<String>> {
    Ok(TICKETS
        .may_load(
            deps.storage,
            (lottery_id, deps.api.addr_validate(addr.as_str())?),
        )?
        .unwrap_or_default())
}

pub fn get_config(deps: Deps) -> StdResult<Config> {
    Ok(CONFIG.load(deps.storage)?)
}

pub fn check_winner(deps: Deps, addr: String, lottery_id: u64) -> StdResult<Vec<TicketResult>> {
    let verify_addr = deps.api.addr_validate(addr.as_str())?;
    let lottery = LOTTERIES.load(deps.storage, lottery_id)?;
    let tickets = TICKETS.load(deps.storage, (lottery_id, verify_addr))?;

    if let Some(w_number) = lottery.winner_number {
        Ok(check_tickets(tickets, w_number))
    } else {
        Ok(vec![])
    }
}
