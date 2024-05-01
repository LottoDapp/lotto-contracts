use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::{Number, RaffleId};
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use openbrush::traits::{AccountId, Storage};

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    current_raffle_id: RaffleId,
    status: Status,
    results: Mapping<RaffleId, Vec<Number>>,
    winners: Mapping<RaffleId, Vec<AccountId>>,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Status {
    #[default]
    NotStarted,
    Ongoing,
    WaitingResults,
    WaitingWinners,
    Closed,
}

#[openbrush::trait_definition]
pub trait Raffle: Storage<Data> {
    /// Start a new raffle
    fn start_new_raffle(&mut self) -> Result<RaffleId, RaffleError> {
        // check the status
        if self.data::<Data>().status != Status::NotStarted
            && self.data::<Data>().status != Status::Closed
        {
            return Err(RaffleError::IncorrectStatus);
        }

        let new_raffle_id = self.data::<Data>().current_raffle_id + 1;
        self.data::<Data>().current_raffle_id = new_raffle_id;
        self.data::<Data>().status = Status::Ongoing;

        Ok(new_raffle_id)
    }

    /// Stop the current raffle
    fn stop_current_raffle(&mut self) -> Result<(), RaffleError> {
        // check the status
        if self.data::<Data>().status != Status::Ongoing {
            return Err(RaffleError::IncorrectStatus);
        }
        // update the status
        self.data::<Data>().status = Status::WaitingResults;
        Ok(())
    }

    #[ink(message)]
    fn get_current_raffle_id(&self) -> RaffleId {
        self.data::<Data>().current_raffle_id
    }

    #[ink(message)]
    fn get_current_status(&self) -> Status {
        self.data::<Data>().status
    }

    #[ink(message)]
    fn get_results(&self, raffle_id: RaffleId) -> Option<Vec<Number>> {
        self.data::<Data>().results.get(raffle_id)
    }

    #[ink(message)]
    fn get_winners(&self, raffle_id: RaffleId) -> Option<Vec<AccountId>> {
        self.data::<Data>().winners.get(raffle_id)
    }

    /// save the results for the current raffle.
    fn set_results(
        &mut self,
        raffle_id: RaffleId,
        results: Vec<Number>,
    ) -> Result<(), RaffleError> {
        // check the raffle number
        if self.data::<Data>().current_raffle_id != raffle_id {
            return Err(RaffleError::IncorrectRaffle);
        }

        // check the status
        if self.data::<Data>().status != Status::WaitingResults {
            return Err(RaffleError::IncorrectStatus);
        }

        match self.data::<Data>().results.get(raffle_id) {
            Some(_) => Err(ExistingResults),
            None => {
                // save the results
                self.data::<Data>().results.insert(raffle_id, &results);
                // update the status
                self.data::<Data>().status = Status::WaitingWinners;
                Ok(())
            }
        }
    }

    /// check if the saved results are the same as the ones given in parameter
    fn ensure_same_results(
        &mut self,
        raffle_id: RaffleId,
        numbers: &[Number],
    ) -> Result<(), RaffleError> {
        // get the correct results for the given raffle
        let result = self
            .data::<Data>()
            .results
            .get(raffle_id)
            .ok_or(DifferentResults)?;

        if result.len() != numbers.len() {
            return Err(DifferentResults);
        }

        for i in 0..numbers.len() {
            if numbers[i] != result[i] {
                return Err(DifferentResults);
            }
        }

        Ok(())
    }

    /// save the winners for the current raffle.
    fn set_winners(
        &mut self,
        raffle_id: RaffleId,
        winners: Vec<AccountId>,
    ) -> Result<(), RaffleError> {
        // check the raffle number
        if self.data::<Data>().current_raffle_id != raffle_id {
            return Err(RaffleError::IncorrectRaffle);
        }

        // check the status
        if self.data::<Data>().status != Status::WaitingWinners {
            return Err(RaffleError::IncorrectStatus);
        }

        match self.data::<Data>().winners.get(raffle_id) {
            Some(_) => Err(ExistingWinners),
            None => {
                // save the result
                self.data::<Data>().winners.insert(raffle_id, &winners);
                // update the status
                self.data::<Data>().status = Status::Closed;
                Ok(())
            }
        }
    }

    /// check if the user can participate to the currnt raffle
    fn can_participate(&mut self) -> Result<(), RaffleError> {
        // check the status
        if self.data::<Data>().status != Status::Ongoing {
            return Err(RaffleError::IncorrectStatus);
        }

        Ok(())
    }
}
