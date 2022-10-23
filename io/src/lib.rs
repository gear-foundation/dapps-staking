#![no_std]

use codec::{Decode, Encode};
use gstd::{prelude::*, ActorId};
use scale_info::TypeInfo;

#[derive(Debug, Clone, Decode, Encode, TypeInfo)]
pub struct InitStaking {
    pub staking_token_address: ActorId,
    pub reward_token_address: ActorId,
    pub distribution_time: u64,
    pub reward_total: u128,
}

#[derive(Debug, Default, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct Staker {
    pub balance: u128,
    pub reward_allowed: u128,
    pub reward_debt: u128,
    pub distributed: u128,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo)]
pub enum StakingAction {
    /// Stake provided tokens amount into staking pool.
    ///
    /// Requirements:
    /// * Provided `amount` should be greater than 0;
    ///
    /// Arguments:
    /// * `amount`: tokens amount.
    ///
    /// On success replies with [`StakingEvent::StakeAccepted`].
    Stake(u128),

    /// Withdraws staked tokens.
    ///
    /// Requirements:
    /// * Provided `amount` should be greater than 0;
    /// * Staker balance should be greater or equal to `amount`;
    ///
    /// Arguments:
    /// * `amount`: tokens amount.
    ///
    /// On success replies with [`StakingEvent::Withdrawn`].
    Withdraw(u128),

    /// Updates staking contract configuration.
    ///
    /// Requirements:
    /// * Should be called by `owner`;
    /// * `reward_total` and `distribution_time`, which provided by [`InitStaking`] can't be 0;
    ///
    /// Arguments:
    /// * `config`: configuration structure [`InitStaking`].
    UpdateStaking(InitStaking),

    /// Picking up staking rewards.
    ///
    /// Requirements:
    /// * Rewards must not be equal to 0;
    ///
    /// On success replies with [`StakingEvent::Reward`].
    GetReward,

    /// Continues the transaction if it fails due to lack of gas
    /// or due to an error in the token contract.
    ///
    /// Requirements:
    /// * `transaction_id` should exists in `transactions` table;
    ///
    /// Arguments:
    /// * `transaction_id`: Identifier of suspended transaction.
    ///
    /// When transaction already processed replies with [`StakingEvent::TransactionProcessed`].
    Continue(u64),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum StakingEvent {
    StakeAccepted(u64, u128),
    Withdrawn(u64, u128),
    Updated,
    Reward(u64, u128),
    TransactionProcessed,
}

#[derive(Debug, Encode, Decode, TypeInfo, PartialEq)]
pub enum StakingState {
    GetStakers,
    GetStaker(ActorId),
}

#[derive(Debug, Encode, Decode, TypeInfo, PartialEq)]
pub enum StakingStateReply {
    Stakers(BTreeMap<ActorId, Staker>),
    Staker(Staker),
}
