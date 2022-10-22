#[cfg(test)]
extern crate std;

mod utils;

use codec::Encode;
use gtest::{Program, System};
use staking_io::*;
use utils::token::*;

const USERS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8];

fn init_staking(sys: &System, st_token_id: u64, rw_token_id: u64) -> Program<'_> {
    let staking = Program::current(sys);

    let res = staking.send(
        USERS[3],
        InitStaking {
            staking_token_address: st_token_id.into(),
            reward_token_address: rw_token_id.into(),
            distribution_time: 10000,
            reward_total: 1000,
        },
    );

    assert!(res.log().is_empty());

    staking
}

fn init_staking_token(sys: &System, id: u64) -> Program<'_> {
    let st_token = Program::ftoken(USERS[3], id, sys);

    st_token.mint(0, USERS[3], USERS[0], 100000, false);
    st_token.check_balance(USERS[0], 100000);

    st_token.mint(1, USERS[3], USERS[4], 10000, false);
    st_token.check_balance(USERS[4], 10000);

    st_token.mint(2, USERS[3], USERS[5], 20000, false);
    st_token.check_balance(USERS[5], 20000);

    st_token.mint(3, USERS[3], USERS[6], 20000, false);
    st_token.check_balance(USERS[6], 20000);

    st_token.mint(4, USERS[3], USERS[7], 20000, false);
    st_token.check_balance(USERS[7], 20000);

    st_token
}

fn init_reward_token(sys: &System, id: u64) -> Program<'_> {
    let rw_token = Program::ftoken(USERS[3], id, sys);

    rw_token.mint(0, USERS[3], USERS[0], 100000, false);
    rw_token.check_balance(USERS[0], 100000);

    rw_token
}

#[test]
fn stake() {
    let sys = System::new();
    sys.init_logger();

    let staking = init_staking(&sys, 1337, 228);

    let res = staking.send(USERS[4], StakingAction::Stake(0));
    assert!(res.main_failed());
}

#[test]
fn update_staking() {
    let sys = System::new();
    sys.init_logger();

    let staking = init_staking(&sys, 1337, 228);

    let res = staking.send(
        USERS[4],
        StakingAction::UpdateStaking(InitStaking {
            staking_token_address: USERS[1].into(),
            reward_token_address: USERS[2].into(),
            distribution_time: 10000,
            reward_total: 1000,
        }),
    );
    assert!(res.main_failed());

    let res = staking.send(
        USERS[3],
        StakingAction::UpdateStaking(InitStaking {
            staking_token_address: USERS[1].into(),
            reward_token_address: USERS[2].into(),
            distribution_time: 10000,
            reward_total: 0,
        }),
    );
    assert!(res.main_failed());

    let res = staking.send(
        USERS[3],
        StakingAction::UpdateStaking(InitStaking {
            staking_token_address: USERS[1].into(),
            reward_token_address: USERS[2].into(),
            distribution_time: 0,
            reward_total: 1000,
        }),
    );
    assert!(res.main_failed());
}

#[test]
fn send_reward() {
    let sys = System::new();
    sys.init_logger();

    let _st_token = init_staking_token(&sys, 1337);
    let _rw_token = init_reward_token(&sys, 228);
    let staking = init_staking(&sys, 1337, 228);

    let res = staking.send(USERS[4], StakingAction::GetReward);
    assert!(res.main_failed());
}

#[test]
fn withdraw() {
    let sys = System::new();
    sys.init_logger();

    let _st_token = init_staking_token(&sys, 1337);
    let _rw_token = init_reward_token(&sys, 228);
    let staking = init_staking(&sys, 1337, 228);

    let res = staking.send(USERS[4], StakingAction::Stake(1500));
    std::println!("{:#?}", res.log());
    assert!(!res.main_failed());
    assert!(res.contains(&(USERS[4], StakingEvent::StakeAccepted(0, 1500).encode())));

    let res = staking.send(USERS[5], StakingAction::Stake(2000));
    assert!(!res.main_failed());
    assert!(res.contains(&(USERS[5], StakingEvent::StakeAccepted(1, 2000).encode())));

    let res = staking.send(USERS[4], StakingAction::Withdraw(0));
    assert!(res.main_failed());

    let res = staking.send(USERS[6], StakingAction::Withdraw(1000));
    assert!(res.main_failed());

    let res = staking.send(USERS[5], StakingAction::Withdraw(5000));
    assert!(res.main_failed());
}
