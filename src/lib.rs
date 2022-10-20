#![no_std]

use codec::{Decode, Encode};
use ft_io::*;
use gstd::{exec, msg, prelude::*, ActorId};
use scale_info::TypeInfo;
use staking_io::*;

#[derive(Debug, Default, Encode, Decode, TypeInfo)]
struct Staking {
    owner: ActorId,
    staking_token_address: ActorId,
    reward_token_address: ActorId,
    tokens_per_stake: u128,
    total_staked: u128,
    distribution_time: u64,
    produced_time: u64,
    reward_total: u128,
    all_produced: u128,
    reward_produced: u128,
    stakers: BTreeMap<ActorId, Staker>,
    transaction_ids: BTreeMap<u128, TransactionStatus>,
}

static mut STAKING: Option<Staking> = None;
const DECIMALS_FACTOR: u128 = 10_u128.pow(20);
const DELAY: u32 = 600_000;

/// Transfers `amount` tokens from `sender` account to `recipient` account.
/// Arguments:
/// * `from`: sender account
/// * `to`: recipient account
/// * `amount`: amount of tokens
async fn transfer_tokens(
    token_address: &ActorId,
    from: &ActorId,
    to: &ActorId,
    amount_tokens: u128,
) {
    msg::send_for_reply(
        *token_address,
        FTAction::Transfer {
            from: *from,
            to: *to,
            amount: amount_tokens,
        },
        0,
    )
    .expect("Error in sending message")
    .await
    .expect("Error in transfer");
}

fn send_delayed_clear(transaction_id: u128) {
    msg::send_delayed(
        exec::program_id(),
        StakingAction::Clear(transaction_id),
        0,
        DELAY,
    )
    .expect("Error in sending a delayled message `FTStorageAction::Clear`");
}

impl Staking {
    /// Calculates the reward produced so far
    fn produced(&mut self) -> u128 {
        let mut elapsed_time = exec::block_timestamp() - self.produced_time;

        if elapsed_time > self.distribution_time {
            elapsed_time = self.distribution_time;
        }

        self.all_produced
            + self.reward_total.saturating_mul(elapsed_time as u128)
                / self.distribution_time as u128
    }

    /// Updates the reward produced so far and calculates tokens per stake
    fn update_reward(&mut self) {
        let reward_produced_at_now = self.produced();

        if reward_produced_at_now > self.reward_produced {
            let produced_new = reward_produced_at_now - self.reward_produced;

            if self.total_staked > 0 {
                self.tokens_per_stake = self
                    .tokens_per_stake
                    .saturating_add((produced_new * DECIMALS_FACTOR) / self.total_staked);
            }

            self.reward_produced = self.reward_produced.saturating_add(produced_new);
        }
    }

    /// Calculates the maximum possible reward
    /// The reward that the depositor would have received if he had initially paid this amount
    /// Arguments:
    /// `amount`: the number of tokens
    fn get_max_reward(&self, amount: u128) -> u128 {
        (amount * self.tokens_per_stake) / DECIMALS_FACTOR
    }

    /// Calculates the reward of the staker that is currently avaiable
    /// The return value cannot be less than zero according to the algorithm
    fn calc_reward(&mut self) -> u128 {
        let staker = self
            .stakers
            .get(&msg::source())
            .unwrap_or_else(|| panic!("calc_reward(): Staker {:?} not found", msg::source()));

        self.get_max_reward(staker.balance) + staker.reward_allowed
            - staker.reward_debt
            - staker.distributed
    }

    /// Updates the staking contract.
    /// Sets the reward to be distributed within distribution time
    /// param 'config' - updated configuration
    fn update_staking(&mut self, config: InitStaking) {
        if msg::source() != self.owner {
            panic!("update_staking(): only the owner can update the staking");
        }

        if config.reward_total == 0 {
            panic!("update_staking(): reward_total is null");
        }

        if config.distribution_time == 0 {
            panic!("update_staking(): distribution_time is null");
        }

        self.staking_token_address = config.staking_token_address;
        self.reward_token_address = config.reward_token_address;
        self.distribution_time = config.distribution_time;

        self.update_reward();
        self.all_produced = self.reward_produced;
        self.produced_time = exec::block_timestamp();
        self.reward_total = config.reward_total;
    }

    /// Stakes the tokens
    /// Arguments:
    /// `amount`: the number of tokens for the stake
    async fn stake(&mut self, transaction_id: u128, amount: u128) {
        if amount == 0 {
            panic!("stake(): amount is null");
        }

        let token_address = self.staking_token_address;

        // Ensure, that user entry exists before async call
        self.stakers.entry(msg::source()).or_insert(Staker {
            reward_debt: 0,
            balance: 0,
            ..Default::default()
        });

        // If `transaction_id` entry already exists, then try exec
        // transfer one more time, but without state changes
        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                // Additional check to ensure, that prev transaction is succeed or failed
                // We must increment id in both of these cases
                assert_eq!(*tx_status, TransactionStatus::InProgress, "Invalid tx id!")
            })
            .or_insert(TransactionStatus::InProgress);

        send_delayed_clear(transaction_id);

        transfer_tokens(&token_address, &msg::source(), &exec::program_id(), amount).await;

        // TODO: Get async reply and possibly change transaction status

        self.update_reward();
        let amount_per_token = self.get_max_reward(amount);

        self.stakers.entry(msg::source()).and_modify(|stake| {
            stake.reward_debt = stake.reward_debt.saturating_add(amount_per_token);
            stake.balance = stake.balance.saturating_add(amount);
        });

        self.total_staked = self.total_staked.saturating_add(amount);

        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                *tx_status = TransactionStatus::Success;
            });

        msg::reply(StakingEvent::StakeAccepted(transaction_id, amount), 0)
            .expect("reply: 'StakeAccepted' error");
    }

    /// Sends reward to the staker
    async fn send_reward(&mut self, transaction_id: u128) {
        self.update_reward();
        let reward = self.calc_reward();

        if reward == 0 {
            panic!("send_reward(): reward is null");
        }

        let token_address = self.reward_token_address;

        // If `transaction_id` entry already exists, then try exec
        // transfer one more time, but without state changes
        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                // Additional check to ensure, that prev transaction is succeed or failed
                // We must increment id in both of these cases
                assert_eq!(*tx_status, TransactionStatus::InProgress, "Invalid tx id!")
            })
            .or_insert(TransactionStatus::InProgress);

        send_delayed_clear(transaction_id);

        transfer_tokens(&token_address, &exec::program_id(), &msg::source(), reward).await;

        // TODO: Get async reply and possibly change transaction status

        self.stakers.entry(msg::source()).and_modify(|stake| {
            stake.distributed = stake.distributed.saturating_add(reward);
        });

        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                *tx_status = TransactionStatus::Success;
            });

        msg::reply(StakingEvent::Reward(transaction_id, reward), 0).expect("reply: 'Reward' error");
    }

    /// Withdraws the staked the tokens
    /// Arguments:
    /// `amount`: the number of withdrawn tokens
    async fn withdraw(&mut self, transaction_id: u128, amount: u128) {
        if amount == 0 {
            panic!("withdraw(): amount is null");
        }

        self.update_reward();
        let amount_per_token = self.get_max_reward(amount);

        let staker = self
            .stakers
            .get_mut(&msg::source())
            .unwrap_or_else(|| panic!("withdraw(): Staker {:?} not found", msg::source()));

        if staker.balance < amount {
            panic!("withdraw(): staker.balance < amount");
        }

        let token_address = self.staking_token_address;

        // If `transaction_id` entry already exists, then try exec
        // transfer one more time, but without state changes
        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                // Additional check to ensure, that prev transaction is succeed or failed
                // We must increment id in both of these cases
                assert_eq!(*tx_status, TransactionStatus::InProgress, "Invalid tx id!")
            })
            .or_insert(TransactionStatus::InProgress);

        send_delayed_clear(transaction_id);

        transfer_tokens(&token_address, &exec::program_id(), &msg::source(), amount).await;

        // TODO: Get async reply and possibly change transaction status

        staker.reward_allowed = staker.reward_allowed.saturating_add(amount_per_token);
        staker.balance = staker.balance.saturating_sub(amount);

        self.total_staked = self.total_staked.saturating_sub(amount);

        self.transaction_ids
            .entry(transaction_id)
            .and_modify(|tx_status| {
                *tx_status = TransactionStatus::Success;
            });

        msg::reply(StakingEvent::Withdrawn(transaction_id, amount), 0)
            .expect("reply: 'Withdrawn' error");
    }

    fn clear_transaction_id(&mut self, transaction_id: u128) {
        self.transaction_ids.remove(&transaction_id);
    }
}

#[gstd::async_main]
async unsafe fn main() {
    let staking = unsafe { STAKING.get_or_insert(Staking::default()) };

    let action: StakingAction = msg::load().expect("Could not load Action");

    match action {
        StakingAction::Stake(transaction_id, amount) => {
            staking.stake(transaction_id, amount).await;
        }

        StakingAction::Withdraw(transaction_id, amount) => {
            staking.withdraw(transaction_id, amount).await;
        }

        StakingAction::UpdateStaking(config) => {
            staking.update_staking(config);
            msg::reply(StakingEvent::Updated, 0).expect("reply: 'Updated' error");
        }

        StakingAction::GetReward(transaction_id) => {
            staking.send_reward(transaction_id).await;
        }

        StakingAction::Clear(transaction_id) => {
            staking.clear_transaction_id(transaction_id);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn init() {
    let config: InitStaking = msg::load().expect("Unable to decode InitConfig");

    let mut staking = Staking {
        owner: msg::source(),
        ..Default::default()
    };

    staking.update_staking(config);
    STAKING = Some(staking);
}

#[no_mangle]
pub unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: StakingState = msg::load().expect("failed to decode input argument");
    let staking = STAKING.get_or_insert(Staking::default());

    let encoded = match query {
        StakingState::GetStakers => StakingStateReply::Stakers(staking.stakers.clone()).encode(),

        StakingState::GetStaker(address) => {
            if let Some(staker) = staking.stakers.get(&address) {
                StakingStateReply::Staker(staker.clone()).encode()
            } else {
                panic!("meta_state(): Staker {:?} not found", address);
            }
        }
    };

    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "Staking",
    init:
        input: InitStaking,
    handle:
        input: StakingAction,
        output: StakingEvent,
    state:
        input: StakingState,
        output: StakingStateReply,
}
