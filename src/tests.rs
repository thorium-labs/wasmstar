use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
    OwnedDeps,
};
use cw_utils::Duration;

use crate::contract::{get_current_lottery, instantiate};
use crate::msg::InstantiateMsg;

const ADMIN_ADDR: &str = "admin";
const NOIS_ADDR: &str = "nois";

fn do_instantaite() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN_ADDR, &[]);
    let env = mock_env();

    let instantiate_msg = InstantiateMsg {
        lottery_interval: Duration::Time(60),
        max_tickets_per_user: 100,
        nois_proxy: NOIS_ADDR.to_string(),
        percentage_per_match: [3, 6, 8, 15, 25, 40],
        ticket_price: coin(1000, "ujuno"),
        treasury_fee: coin(1000, "ujuno"),
    };

    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    deps
}

#[test]
fn cannot_buy_tickets_when_lottery_is_expired_or_not_open() {
    let deps = do_instantaite();
    let res = get_current_lottery(deps.as_ref()).unwrap();

    println!("{:?}", res);
}
