use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::{Number, LOTTO_MANAGER_ROLE};
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use ink::storage::traits::StorageLayout;
use openbrush::contracts::access_control::access_control;
use openbrush::traits::{AccountId, Storage};

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    current_raffle: u32,
    status: Status,
    results: Mapping<u32, Vec<Number>>,
    winners: Mapping<u32, Vec<AccountId>>,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub enum Status {
    #[default]
    NotStarted,
    Ongoing,
    WaitingResults,
    WaitingWinners,
    Closed,
}

#[openbrush::trait_definition]
pub trait Raffle: Internal + Storage<Data> + access_control::Internal {

    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn start_raffle(&mut self, num_raffle: u32) -> Result<(), RaffleError> {
        // check the status
        if self.data::<Data>().status != Status::NotStarted
            && self.data::<Data>().status != Status::Closed
        {
            return Err(RaffleError::IncorrectStatus);
        }

        self.data::<Data>().current_raffle = num_raffle;
        self.data::<Data>().status = Status::Ongoing;
        Ok(())
    }

    fn stop_raffle(&mut self) -> Result<(), RaffleError> {
        // check the status
        if self.data::<Data>().status != Status::Ongoing {
            return Err(RaffleError::IncorrectStatus);
        }
        // update the status
        self.data::<Data>().status = Status::WaitingResults;
        Ok(())
    }

    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn delete_raffle(&mut self, num_raffle: u32) -> Result<(), RaffleError> {
        // TODO check if the raffle is not the current one

        self.data::<Data>().results.remove(num_raffle);
        Ok(())
    }

    #[ink(message)]
    fn get_current_raffle(&self) -> u32 {
        self.data::<Data>().current_raffle
    }

    #[ink(message)]
    fn get_status(&self) -> Status {
        self.data::<Data>().status
    }

    #[ink(message)]
    fn get_results(&self, num_raffle: u32) -> Option<Vec<Number>> {
        self.data::<Data>().results.get(num_raffle)
    }

    #[ink(message)]
    fn get_winners(&self, num_raffle: u32) -> Option<Vec<AccountId>> {
        self.data::<Data>().winners.get(num_raffle)
    }

    fn set_results(
        &mut self,
        num_raffle: u32,
        results: Vec<Number>,
    ) -> Result<(), RaffleError> {

        // check the raffle number
        if self.data::<Data>().current_raffle != num_raffle {
            return Err(RaffleError::IncorrectRaffle);
        }

        // check the status
        if self.data::<Data>().status != Status::WaitingResults {
            return Err(RaffleError::IncorrectStatus);
        }

        match self.data::<Data>().results.get(num_raffle) {
            Some(_) => Err(ExistingResults),
            None => {
                // save the result
                self.data::<Data>().results.insert(num_raffle, &results);
                // emmit the event
                self.emit_results(num_raffle, results);
                // update the status
                self.data::<Data>().status = Status::WaitingWinners;
                Ok(())
            }
        }
    }

    fn set_winners(
        &mut self,
        num_raffle: u32,
        winners: Vec<AccountId>,
    ) -> Result<(), RaffleError> {

        // check the raffle number
        if self.data::<Data>().current_raffle != num_raffle {
            return Err(RaffleError::IncorrectRaffle);
        }

        // check the status
        if self.data::<Data>().status != Status::WaitingResults {
            return Err(RaffleError::IncorrectStatus);
        }

        match self.data::<Data>().winners.get(num_raffle) {
            Some(_) => Err(ExistingWinners),
            None => {
                // save the result
                self.data::<Data>().winners.insert(num_raffle, &winners);
                // emmit the event
                self.emit_winners(num_raffle, winners);
                // update the status
                self.data::<Data>().status = Status::Closed;
                Ok(())
            }
        }
    }

    fn inner_participate(&mut self, numbers: Vec<Number>) -> Result<(), RaffleError> {

        // check the status
        if self.data::<Data>().status != Status::Ongoing {
            return Err(RaffleError::IncorrectStatus);
        }

        let participant = Self::env().caller();
        let num_raffle = self.data::<Data>().current_raffle;

        // save the participant with an event
        self.emit_participation_registered(num_raffle, participant, numbers);

        Ok(())
    }
}

#[openbrush::trait_definition]
pub trait Internal {
    fn emit_participation_registered(
        &self,
        num_raffle: u32,
        participant: AccountId,
        numbers: Vec<Number>,
    );
    fn emit_results(&self, num_raffle: u32, result: Vec<Number>);
    fn emit_winners(&self, num_raffle: u32, winners: Vec<AccountId>);
}
