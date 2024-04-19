use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::{Number, LOTTO_MANAGER_ROLE, RaffleId};
use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use ink::storage::traits::StorageLayout;
use openbrush::contracts::access_control::access_control;
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
    fn get_current_raffle_id(&self) -> RaffleId {
        self.data::<Data>().current_raffle_id
    }

    #[ink(message)]
    fn get_status(&self) -> Status {
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
                // save the result
                self.data::<Data>().results.insert(raffle_id, &results);
                // emmit the event
                self.emit_results(raffle_id, results);
                // update the status
                self.data::<Data>().status = Status::WaitingWinners;
                Ok(())
            }
        }
    }

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
        if self.data::<Data>().status != Status::WaitingResults {
            return Err(RaffleError::IncorrectStatus);
        }

        match self.data::<Data>().winners.get(raffle_id) {
            Some(_) => Err(ExistingWinners),
            None => {
                // save the result
                self.data::<Data>().winners.insert(raffle_id, &winners);
                // emmit the event
                self.emit_winners(raffle_id, winners);
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
        let raffle_id = self.data::<Data>().current_raffle_id;

        // save the participant with an event
        self.emit_participation_registered(raffle_id, participant, numbers);

        Ok(())
    }
}

#[openbrush::trait_definition]
pub trait Internal {
    fn emit_participation_registered(
        &self,
        raffle_id: RaffleId,
        participant: AccountId,
        numbers: Vec<Number>,
    );
    fn emit_results(&self, raffle_id: RaffleId, result: Vec<Number>);
    fn emit_winners(&self, raffle_id: RaffleId, winners: Vec<AccountId>);
}
