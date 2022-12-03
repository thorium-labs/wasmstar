use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    to_binary, Addr, CosmosMsg, HexBinary, OwnedDeps, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw_utils::{Duration, Expiration};
use nois::{ints_in_range, NoisCallback, ProxyExecuteMsg};

use crate::{
    contract::{
        buy_tickets, check_winner, execute_draw, get_config, get_current_draw, get_draw,
        get_tickets, instantiate, receive_randomness,
    },
    error::ContractError,
    helpers::{calculate_matches, calculate_prize_distribution, create_next_draw},
    state::{Draw, DRAWS},
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
        draw_interval: Duration::Time(60),
        max_tickets_per_user: MAX_TICKETS,
        nois_proxy: NOIS_ADDR.to_string(),
        percentage_per_match: [3, 6, 8, 15, 25, 40],
        ticket_price: coin(TICKET_PRICE, DENOM),
        treasury_fee: 3,
    };

    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    deps
}

#[test]
fn cannot_buy_tickets_when_draw_is_not_open() {
    let mut deps = do_instantaite();

    DRAWS
        .update(deps.as_mut().storage, 1, |d| -> StdResult<Draw> {
            let mut draw = d.unwrap();
            draw.status = Status::Claimable;
            Ok(draw)
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

    assert_eq!(err, ContractError::DrawIsNotOpen);
}

#[test]
fn cannot_buy_tickets_when_draw_is_expired() {
    let mut deps = do_instantaite();

    DRAWS
        .update(deps.as_mut().storage, 1, |d| -> StdResult<Draw> {
            let mut draw = d.unwrap();
            draw.end_time = Expiration::AtTime(Timestamp::from_seconds(0));
            Ok(draw)
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

    assert_eq!(err, ContractError::DrawIsNotOpen);
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

    let draw = get_current_draw(deps.as_ref()).unwrap();

    assert_eq!(draw.prize_per_match, Some([Uint128::zero(); 6]));
    assert_eq!(draw.total_tickets, 0);
    assert_eq!(draw.total_prize.amount, Uint128::zero());

    buy_tickets(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[coin(TICKET_PRICE, DENOM)]),
        tickets.clone(),
        1,
    )
    .unwrap();

    let draw = get_current_draw(deps.as_ref()).unwrap();
    let config = get_config(deps.as_ref()).unwrap();

    let expected_prize_per_match =
        calculate_prize_distribution(draw.total_prize.amount.clone(), config.percentage_per_match);

    assert_eq!(draw.total_tickets, 1);
    assert_eq!(
        draw.total_prize.amount,
        Uint128::from(TICKET_PRICE * tickets.len() as u128)
    );
    assert_eq!(draw.prize_per_match, Some(expected_prize_per_match))
}

#[test]
fn cannot_execute_draw_if_is_not_expired() {
    let mut deps = do_instantaite();

    let err = execute_draw(
        deps.as_mut(),
        mock_env(),
        mock_info(PARTICIPANT_ADDR, &[]),
        1,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::DrawStillOpen);
}

#[test]
fn execute_draw_should_work() {
    let mut deps = do_instantaite();
    DRAWS
        .update(deps.as_mut().storage, 1, |d| -> StdResult<Draw> {
            let mut draw = d.unwrap();
            draw.end_time = Expiration::AtTime(Timestamp::from_seconds(0));
            Ok(draw)
        })
        .unwrap();

    let resp = execute_draw(
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

    let draw = get_draw(deps.as_ref(), 1).unwrap().unwrap();

    assert_eq!(draw.status, Status::Pending);
}

#[test]
fn only_nois_can_execute_receive_randomness() {
    let mut deps = do_instantaite();
    let randomness = HexBinary::from(vec![
        88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9, 237, 27,
        215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
    ]);

    let err = receive_randomness(
        deps.as_mut(),
        mock_info(PARTICIPANT_ADDR, &[]),
        mock_env(),
        NoisCallback {
            job_id: "1".to_string(),
            randomness,
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized);
}

#[test]
fn execute_receive_randomness_should_work() {
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

    DRAWS
        .update(deps.as_mut().storage, 1, |draw| -> StdResult<Draw> {
            let mut draw = draw.unwrap();
            draw.status = Status::Pending;
            Ok(draw)
        })
        .unwrap();

    receive_randomness(
        deps.as_mut(),
        mock_info("nois", &[]),
        mock_env(),
        NoisCallback {
            job_id: "1".to_string(),
            randomness: randomness.clone(),
        },
    )
    .unwrap();

    let draw = get_draw(deps.as_ref(), 1).unwrap().unwrap();

    assert_eq!(draw.status, Status::Claimable);
    assert_eq!(draw.winner_number, Some(winner_number));
    assert_eq!(draw.total_tickets, 1);
    assert_eq!(
        draw.total_prize,
        coin(TICKET_PRICE * tickets.len() as u128, DENOM)
    );
}

#[test]
fn calculate_matches_should_return_match_same_position() {
    let winning_ticket = "123456";
    let ticket = "234561";

    let matches = calculate_matches(winning_ticket, ticket);
    assert_eq!(matches, 0);

    let winning_ticket = "123456";
    let ticket = "143456";

    let matches = calculate_matches(winning_ticket, ticket);
    assert_eq!(matches, 1);

    let winning_ticket = "123456";
    let ticket = "123456";

    let matches = calculate_matches(winning_ticket, ticket);
    assert_eq!(matches, 6)
}

#[test]
fn check_winner_when_empty() {
    let mut deps = do_instantaite();
    create_next_draw(deps.as_mut(), &mock_env(), Uint128::zero()).unwrap();
    DRAWS
        .update(deps.as_mut().storage, 1, |d| -> StdResult<Draw> {
            let mut draw = d.unwrap();
            draw.winner_number = Some("123456".to_string());
            Ok(draw)
        })
        .unwrap();

    let result = check_winner(deps.as_ref(), "addr".to_string(), 1).unwrap();
    assert_eq!(result, vec![]);
}
