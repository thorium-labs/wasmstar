use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    to_binary, CosmosMsg, HexBinary, OwnedDeps, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw_utils::{Duration, Expiration};
use nois::{ints_in_range, NoisCallback, ProxyExecuteMsg};

use crate::{
    contract::{
        buy_tickets, execute_lottery, get_config, get_current_lottery, get_lottery, get_tickets,
        instantiate, random_callback,
    },
    error::ContractError,
    helpers::calculate_prize_distribution,
    state::{Lottery, LOTTERIES},
};
use crate::{msg::InstantiateMsg, state::Status};

const ADMIN_ADDR: &str = "admin";
const PARTICIPANT_ADDR: &str = "participant";
const NOIS_ADDR: &str = "nois";
const DENOM: &str = "udenom";
const MAX_TICKETS: u32 = 10;
const TICKET_PRICE: u128 = 1000;

fn do_instantaite() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN_ADDR, &[]);
    let env = mock_env();

    let instantiate_msg = InstantiateMsg {
        lottery_interval: Duration::Time(60),
        max_tickets_per_user: MAX_TICKETS,
        nois_proxy: NOIS_ADDR.to_string(),
        percentage_per_match: [3, 6, 8, 15, 25, 40],
        ticket_price: coin(TICKET_PRICE, DENOM),
        treasury_fee: coin(1000, DENOM),
    };

    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    deps
}

#[test]
fn cannot_buy_tickets_when_lottery_is_not_open() {
    let mut deps = do_instantaite();

    LOTTERIES
        .update(deps.as_mut().storage, 1, |l| -> StdResult<Lottery> {
            let mut lottery = l.unwrap();
            lottery.status = Status::Claimable;
            Ok(lottery)
        })
        .unwrap();

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        vec!["".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::LotteryIsNotOpen);
}

#[test]
fn cannot_buy_tickets_when_lottery_is_expired() {
    let mut deps = do_instantaite();

    LOTTERIES
        .update(deps.as_mut().storage, 1, |l| -> StdResult<Lottery> {
            let mut lottery = l.unwrap();
            lottery.end_time = Expiration::AtTime(Timestamp::from_seconds(0));
            Ok(lottery)
        })
        .unwrap();

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        vec!["".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::LotteryIsNotOpen);
}

#[test]
fn cannot_buy_tickets_than_max_tickets_per_user() {
    let mut deps = do_instantaite();

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        vec!["".to_string(); MAX_TICKETS as usize + 1],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::MaxTicketsPerUserExceeded);
}

#[test]
fn cannot_buy_tickets_when_sending_invalid_tickets() {
    let mut deps = do_instantaite();

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        vec!["invalid".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidTicket);

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        vec!["1234".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidTicket);
}

#[test]
fn cannot_buy_tickets_when_not_provide_enough_funds() {
    let mut deps = do_instantaite();

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(TICKET_PRICE, "other")]),
        vec!["123456".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidDenom);

    let err = buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(0, DENOM)]),
        vec!["123456".to_string()],
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidAmount);
}

#[test]
fn once_purchased_ticktes_should_be_able_to_query_them() {
    let mut deps = do_instantaite();
    let tickets = vec!["123456".to_string()];

    buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(TICKET_PRICE, DENOM)]),
        tickets.clone(),
        1,
    )
    .unwrap();

    let btickets = get_tickets(deps.as_ref(), 1, PARTICIPANT_ADDR.to_string()).unwrap();

    assert_eq!(tickets, btickets);
}

#[test]
fn once_purchased_ticktes_it_should_update_prizes() {
    let mut deps = do_instantaite();
    let tickets = vec!["123456".to_string()];

    let lottery = get_current_lottery(deps.as_ref()).unwrap();

    assert_eq!(lottery.prize_per_match, None);
    assert_eq!(lottery.total_tickets, 0);
    assert_eq!(lottery.total_prize.amount, Uint128::zero());

    buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(TICKET_PRICE, DENOM)]),
        tickets.clone(),
        1,
    )
    .unwrap();

    let lottery = get_current_lottery(deps.as_ref()).unwrap();
    let config = get_config(deps.as_ref()).unwrap();

    let expected_prize_per_match = calculate_prize_distribution(
        lottery.total_prize.amount.clone(),
        config.percentage_per_match,
    );

    assert_eq!(lottery.total_tickets, 1);
    assert_eq!(
        lottery.total_prize.amount,
        Uint128::from(TICKET_PRICE * tickets.len() as u128)
    );
    assert_eq!(lottery.prize_per_match, Some(expected_prize_per_match))
}

#[test]
fn cannot_execute_lottery_if_is_not_expired() {
    let mut deps = do_instantaite();

    let err = execute_lottery(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::LotteryStillOpen);
}

#[test]
fn execute_lottery_should_work() {
    let mut deps = do_instantaite();
    LOTTERIES
        .update(deps.as_mut().storage, 1, |l| -> StdResult<Lottery> {
            let mut lottery = l.unwrap();
            lottery.end_time = Expiration::AtTime(Timestamp::from_seconds(0));
            Ok(lottery)
        })
        .unwrap();

    let resp = execute_lottery(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        1,
    )
    .unwrap();

    let message = resp.messages.first().unwrap();

    assert_eq!(
        message.msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nois".to_string(),
            funds: vec![],
            msg: to_binary(&ProxyExecuteMsg::GetNextRandomness {
                job_id: "1".to_string(),
            })
            .unwrap(),
        })
    );

    let lottery = get_lottery(deps.as_ref(), 1).unwrap().unwrap();

    assert_eq!(lottery.status, Status::Pending);
}

#[test]
fn only_nois_can_execute_random_callback() {
    let mut deps = do_instantaite();
    let randomness = HexBinary::from(vec![
        88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9, 237, 27,
        215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
    ]);

    let err = random_callback(
        deps.as_mut(),
        mock_info(PARTICIPANT_ADDR, &[]),
        NoisCallback {
            job_id: "1".to_string(),
            randomness,
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized);
}

#[test]
fn execute_random_callback_should_work() {
    let mut deps = do_instantaite();
    let randomness = HexBinary::from(vec![
        88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9, 237, 27,
        215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
    ]);

    let random_result: [u8; 6] = ints_in_range(randomness.to_array().unwrap(), 0..=9);
    let winner_number = random_result
        .into_iter()
        .fold(String::new(), |acc, x| acc + &x.to_string());

    let tickets = vec!["123456".to_string()];

    buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(TICKET_PRICE, DENOM)]),
        tickets.clone(),
        1,
    )
    .unwrap();

    random_callback(
        deps.as_mut(),
        mock_info("nois", &[]),
        NoisCallback {
            job_id: "1".to_string(),
            randomness: randomness.clone(),
        },
    )
    .unwrap();

    let lottery = get_lottery(deps.as_ref(), 1).unwrap().unwrap();

    assert_eq!(lottery.status, Status::Claimable);
    assert_eq!(lottery.winner_number, Some(winner_number));
    assert_eq!(lottery.total_tickets, 1);
    assert_eq!(
        lottery.total_prize,
        coin(TICKET_PRICE * tickets.len() as u128, DENOM)
    );
}
