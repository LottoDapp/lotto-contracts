use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::{LOTTO_MANAGER_ROLE, NUMBER};
use ink::prelude::vec::Vec;
use ink::storage::traits::StorageLayout;
use openbrush::contracts::access_control::access_control;
use openbrush::traits::Storage;

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    config: Option<Config>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct Config {
    nb_numbers: u8,
    min_number: NUMBER,
    max_number: NUMBER,
}

#[openbrush::trait_definition]
pub trait RaffleConfig: Storage<Data> + access_control::Internal {
    #[ink(message)]
    #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
    fn set_config(&mut self, config: Config) -> Result<(), RaffleError> {
        // check the config
        if config.nb_numbers == 0 {
            return Err(IncorrectConfig);
        }

        if config.min_number >= config.max_number {
            return Err(IncorrectConfig);
        }

        self.data::<Data>().config = Some(config);
        Ok(())
    }

    #[ink(message)]
    fn get_config(&self) -> Option<Config> {
        self.data::<Data>().config
    }

    /// return the config and throw an error of the config is missing
    fn ensure_config(&mut self) -> Result<Config, RaffleError> {
        match self.data::<Data>().config {
            None => Err(ConfigNotSet),
            Some(config) => Ok(config),
        }
    }

    fn check_numbers(&mut self, numbers: &Vec<NUMBER>) -> Result<(), RaffleError> {
        // check if the config is set
        let config = self.ensure_config()?;

        // check the numbers
        let nb_numbers = numbers.len();

        if nb_numbers != config.nb_numbers as usize {
            return Err(IncorrectNbNumbers);
        }

        for number in numbers.iter() {
            if *number > config.max_number || *number < config.min_number {
                return Err(IncorrectNumbers);
            }
        }

        Ok(())
    }
}
