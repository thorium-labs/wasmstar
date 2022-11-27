#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coin, ensure_eq, to_binary, wasm_execute, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdResult, Uint128,
};
use nois::{ints_in_range, NoisCallback, ProxyExecuteMsg};
use std::ops::{Add, Mul};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::{
    calculate_prize_distribution, calculate_tickets_prize, calculate_winner_per_match,
    check_tickets, create_next_draw, ensure_is_enough_funds_to_cover_tickets,
    ensure_ticket_is_valid, increase_prize_with_accumulative_pot,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfigMsg};
use crate::state::{
    Config, Draw, Status, TicketResult, CONFIG, DRAWS, DRAWS_INDEX, TICKETS, WINNERS,
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
        interval: msg.draw_interval,
        ticket_price: msg.ticket_price,
        nois_proxy: nois_proxy_addr,
        treasury_fee: msg.treasury_fee,
        percentage_per_match: msg.percentage_per_match,
        max_tickets_per_user: msg.max_tickets_per_user,
    };

    CONFIG.save(deps.storage, &config)?;
    DRAWS_INDEX.save(deps.storage, &0)?;

    create_next_draw(deps, env)?;

    Ok(Response::new().add_attribute("action", "/superstar.v1.MsgInstantiateContract"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BuyTicket { tickets, draw_id } => {
            buy_tickets(deps, env, info, tickets, draw_id)
        }
        ExecuteMsg::ClaimPrize { draw_id } => claim_prize(deps, info, draw_id),
        ExecuteMsg::ExecuteDraw { id } => execute_draw(deps, env, info, id),
        ExecuteMsg::Receive { callback } => receive_randomness(deps, info, callback),
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, info, new_config),
    }
}

pub fn buy_tickets(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tickets: Vec<String>,
    draw_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut draw = DRAWS.load(deps.storage, draw_id)?;

    if draw.status != Status::Open || draw.end_time.is_expired(&env.block) {
        return Err(ContractError::DrawIsNotOpen);
    }

    let mut tickets_bought = TICKETS
        .may_load(deps.storage, (draw_id, info.sender.clone()))?
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

    TICKETS.save(deps.storage, (draw_id, info.sender), &tickets_bought)?;

    draw.total_tickets = draw.total_tickets.add(u64::from(n_tickets));
    draw.total_prize.amount += required_funds.amount;

    draw.prize_per_match = Some(calculate_prize_distribution(
        draw.total_prize.amount.clone(),
        config.percentage_per_match.clone(),
    ));

    DRAWS.save(deps.storage, draw_id, &draw)?;

    Ok(Response::new()
        .add_attribute("action", "/superstar.v1.MsgBuyTickets")
        .add_attribute("total_tickets", draw.total_tickets.to_string())
        .add_attribute("total_prize", draw.total_prize.amount))
}

pub fn claim_prize(
    deps: DepsMut,
    info: MessageInfo,
    draw_id: u64,
) -> Result<Response, ContractError> {
    let draw = DRAWS.load(deps.storage, draw_id)?;

    if draw.status != Status::Claimable {
        return Err(ContractError::DrawIsNotClaimable);
    }

    let tickets = TICKETS.load(deps.storage, (draw_id, info.sender.clone()))?;

    let t_result = check_tickets(
        tickets,
        draw.winner_number.ok_or(ContractError::InvalidRandomness)?,
    );

    let prize = calculate_tickets_prize(
        t_result,
        draw.prize_per_match.unwrap_or_default(),
        draw.winners_per_match.unwrap_or_default(),
        draw.ticket_price.clone().denom,
    );

    if prize.amount.is_zero() {
        return Err(ContractError::NoPrizeToClaim);
    }

    if WINNERS
        .may_load(deps.storage, (draw_id, info.sender.clone()))?
        .is_some()
    {
        return Err(ContractError::AlreadyClaimed);
    }

    WINNERS.save(
        deps.storage,
        (draw_id, info.sender.clone()),
        &draw.ticket_price,
    )?;

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![prize.clone()],
        }))
        .add_attribute("action", "/superstar.v1.MsgClaimPrize")
        .add_attribute("draw_id", draw_id.to_string())
        .add_attribute("winner", info.sender.to_string())
        .add_attribute("prize", prize.to_string()))
}

pub fn execute_draw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let mut draw = DRAWS.load(deps.storage, id.clone())?;

    if !draw.end_time.is_expired(&env.block) {
        return Err(ContractError::DrawStillOpen);
    }

    let config = CONFIG.load(deps.storage)?;

    let msg = wasm_execute(
        config.nois_proxy,
        &ProxyExecuteMsg::GetNextRandomness {
            job_id: id.to_string(),
        },
        info.funds,
    )?;

    draw.status = Status::Pending;

    DRAWS.save(deps.storage, id, &draw)?;
    create_next_draw(deps, env.clone())?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "/superstar.v1.MsgExecuteDraw")
        .add_attribute("draw_id", id.to_string())
        .add_attribute("executed_at", env.block.time.seconds().to_string()))
}

pub fn receive_randomness(
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

    let draw_id = callback
        .job_id
        .parse::<u64>()
        .expect("error parsing job_id");

    let purchases = TICKETS
        .prefix(draw_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(Addr, Vec<String>)>>>()?;

    let mut draw = DRAWS.load(deps.storage, draw_id)?;

    let winners_per_match = calculate_winner_per_match(purchases, winner_number.clone());

    draw.winner_number = Some(winner_number.clone());
    draw.status = Status::Claimable;
    draw.winners_per_match = Some(winners_per_match.clone());

    DRAWS.save(deps.storage, draw_id, &draw)?;

    let accumulative_pot = draw
        .prize_per_match
        .unwrap()
        .iter()
        .fold(Uint128::zero(), |acc, x| acc.add(x.clone()));

    increase_prize_with_accumulative_pot(deps, draw_id.add(1), accumulative_pot.clone())?;

    Ok(Response::new()
        .add_attribute("action", "/superstar.v1.MsgReceiveRandomness")
        .add_attribute("draw_id", draw_id.to_string())
        .add_attribute(
            "winner_number",
            draw.winner_number.unwrap_or_default().to_string(),
        ))
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

    Ok(Response::new().add_attribute("action", "/superstar.v1.MsgUpdateConfig"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCurrentDraw {} => to_binary(&get_current_draw(deps)?),
        QueryMsg::GetDraw { id } => to_binary(&get_draw(deps, id)?),
        QueryMsg::CheckWinner { addr, draw_id } => to_binary(&check_winner(deps, addr, draw_id)?),
        QueryMsg::GetTickets { addr, draw_id } => to_binary(&get_tickets(deps, draw_id, addr)?),
        QueryMsg::GetConfig {} => to_binary(&get_config(deps)?),
    }
}

pub fn get_current_draw(deps: Deps) -> StdResult<Draw> {
    Ok(DRAWS.load(deps.storage, DRAWS_INDEX.load(deps.storage)?)?)
}

pub fn get_draw(deps: Deps, id: u64) -> StdResult<Option<Draw>> {
    Ok(DRAWS.may_load(deps.storage, id)?)
}

pub fn get_tickets(deps: Deps, draw_id: u64, addr: String) -> StdResult<Vec<String>> {
    Ok(TICKETS
        .may_load(
            deps.storage,
            (draw_id, deps.api.addr_validate(addr.as_str())?),
        )?
        .unwrap_or_default())
}

pub fn get_config(deps: Deps) -> StdResult<Config> {
    Ok(CONFIG.load(deps.storage)?)
}

pub fn check_winner(deps: Deps, addr: String, draw_id: u64) -> StdResult<Vec<TicketResult>> {
    let draw = DRAWS.load(deps.storage, draw_id)?;
    let tickets = TICKETS.load(
        deps.storage,
        (draw_id, deps.api.addr_validate(addr.as_str())?),
    )?;

    if let Some(w_number) = draw.winner_number {
        Ok(check_tickets(tickets, w_number))
    } else {
        Ok(vec![])
    }
}
