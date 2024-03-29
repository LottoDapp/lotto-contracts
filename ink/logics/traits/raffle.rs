use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::LOTTO_MANAGER_ROLE;
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use openbrush::contracts::access_control::access_control;
use openbrush::traits::{AccountId, Storage};

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    current_raffle: u32,
    results: Mapping<u32, Vec<u8>>,
    winners: Mapping<u32, Vec<AccountId>>,
}

#[openbrush::trait_definition]
pub trait Raffle: Internal + Storage<Data> + access_control::Internal {
    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn start_raffle(&mut self, num_raffle: u32) -> Result<(), RaffleError> {
        // TODO check if the raffle doesn't exist
        self.data::<Data>().current_raffle = num_raffle;
        Ok(())
    }

    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn stop_raffle(&mut self) -> Result<(), RaffleError> {
        self.data::<Data>().current_raffle = 0;
        Ok(())
    }

    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn delete_raffle(&mut self, num_raffle: u32) -> Result<(), RaffleError> {
        // TODO check if the raffle is not the current one

        self.data::<Data>().results.remove(num_raffle);
        self.data::<Data>().results.remove(num_raffle);
        Ok(())
    }

    #[ink(message)]
    fn get_current_raffle(&self) -> u32 {
        self.data::<Data>().current_raffle
    }

    #[ink(message)]
    fn get_results(&self, num_raffle: u32) -> Option<Vec<u8>> {
        self.data::<Data>().results.get(num_raffle)
    }

    #[ink(message)]
    fn get_winners(&self, num_raffle: u32) -> Option<Vec<AccountId>> {
        self.data::<Data>().winners.get(num_raffle)
    }

    fn inner_set_results(&mut self, num_raffle: u32, results: Vec<u8>) -> Result<(), RaffleError> {
        // TODO check if the raffle exists and it's stopped

        match self.data::<Data>().results.get(num_raffle) {
            Some(_) => Err(ExistingResults),
            None => {
                // save the result
                self.data::<Data>().results.insert(num_raffle, &results);
                // emmit the event
                self.emit_results(num_raffle, results);
                Ok(())
            }
        }
    }

    fn inner_set_winners(
        &mut self,
        num_raffle: u32,
        winners: Vec<AccountId>,
    ) -> Result<(), RaffleError> {
        // TODO check if the raffle exists and it's stopped and the results are known

        match self.data::<Data>().winners.get(num_raffle) {
            Some(_) => Err(ExistingWinners),
            None => {
                // save the result
                self.data::<Data>().winners.insert(num_raffle, &winners);
                // emmit the event
                self.emit_winners(num_raffle, winners);
                Ok(())
            }
        }
    }

    fn inner_participate(&mut self, numbers: Vec<u8>) -> Result<(), RaffleError> {
        // TODO check if a raffle is started

        let participant = Self::env().caller();
        let num_raffle = self.data::<Data>().current_raffle;

        // save the participant with an event
        self.emit_participation_registered(num_raffle, participant, numbers);

        Ok(())
    }
}

#[openbrush::trait_definition]
pub trait Internal {
    fn emit_participation_registered(&self, num_raffle: u32, participant: AccountId, numbers: Vec<u8>);
    fn emit_results(&self, num_raffle: u32, result: Vec<u8>);
    fn emit_winners(&self, num_raffle: u32, winners: Vec<AccountId>);
}
