use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use ink::prelude::vec::Vec;
use openbrush::storage::Mapping;
use openbrush::traits::{AccountId, Balance, Storage};

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    pending_rewards: Mapping<AccountId, Balance>,
    total_pending_rewards: Balance,
}

#[openbrush::trait_definition]
pub trait RewardManager: Internal + Storage<Data> {

    #[ink(message, payable)]
    fn fund(&mut self) -> Result<(), RaffleError> {
        Ok(())
    }

    fn add_winners(&mut self, accounts: Vec<AccountId>) -> Result<(), RaffleError> {
        let mut total_pending_rewards = self.data::<Data>().total_pending_rewards;

        let reward = Self::env()
            .balance()
            .checked_sub(total_pending_rewards)
            .ok_or(AddOverFlow)?
            .checked_div(accounts.len() as u128)
            .ok_or(DivByZero)?;

        // iterate on the accounts (the winners)
        for account in accounts {
            // compute the new rewards for this winner
            let new_reward = match self.data::<Data>().pending_rewards.get(&account) {
                Some(existing_reward) => existing_reward.checked_add(reward).ok_or(AddOverFlow)?,
                _ => reward,
            };

            // add the pending rewards for this account
            self.data::<Data>()
                .pending_rewards
                .insert(&account, &new_reward);

            self.emit_pending_reward_event(account, reward);

            // update the total pending rewards
            total_pending_rewards = total_pending_rewards
                .checked_add(reward)
                .ok_or(AddOverFlow)?;
        }
        // update the storage
        self.data::<Data>().total_pending_rewards = total_pending_rewards;
        Ok(())
    }

    /// return the total pending reward
    #[ink(message)]
    fn get_total_pending_rewards(&mut self) -> Balance {
        self.data::<Data>().total_pending_rewards
    }

    /// return true if the current account has pending rewards
    #[ink(message)]
    fn has_pending_rewards(&self) -> bool {
        let from = Self::env().caller();
        self.inner_has_pending_rewards_from(from)
    }

    /// return true if the given account has pending rewards
    #[ink(message)]
    fn has_pending_rewards_from(&mut self, from: AccountId) -> bool {
        self.inner_has_pending_rewards_from(from)
    }

    fn inner_has_pending_rewards_from(&self, from: AccountId) -> bool {
        self.data::<Data>().pending_rewards.contains(&from)
    }

    /// return the pending rewards for a given account.
    #[ink(message)]
    fn get_pending_rewards_from(
        &mut self,
        from: AccountId,
    ) -> Option<Balance> {
        self.data::<Data>().pending_rewards.get(&from)
    }

    /// claim all pending rewards for the current account
    /// After claiming, there is not anymore pending rewards for this account
    #[ink(message)]
    fn claim(&mut self) -> Result<(), RaffleError> {
        let from = Self::env().caller();
        self.inner_claim_from(from)
    }

    /// claim all pending rewards for the given account
    /// After claiming, there is not anymore pending rewards for this account
    #[ink(message)]
    fn claim_from(&mut self, from: AccountId) -> Result<(), RaffleError> {
        self.inner_claim_from(from)
    }

    fn inner_claim_from(&mut self, from: AccountId) -> Result<(), RaffleError> {
        // get all pending rewards for this account
        match self.data::<Data>().pending_rewards.get(&from) {
            Some(pending_rewards) => {
                // transfer the amount
                Self::env()
                    .transfer(from, pending_rewards)
                    .map_err(|_| TransferError)?;
                // emmit the event
                self.emit_rewards_claimed_event(from, pending_rewards);
                // remove the pending rewards
                self.data::<Data>().pending_rewards.remove(&from);

                // update the total pending rewards
                self.data::<Data>().total_pending_rewards = self
                    .data::<Data>()
                    .total_pending_rewards
                    .checked_sub(pending_rewards)
                    .ok_or(SubOverFlow)?;

                Ok(())
            }
            _ => Err(NoReward),
        }
    }
}

#[openbrush::trait_definition]
pub trait Internal {
    fn emit_pending_reward_event(&self, account: AccountId, amount: Balance);
    fn emit_rewards_claimed_event(&self, account: AccountId, amount: Balance);
}
