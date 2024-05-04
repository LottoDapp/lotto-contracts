use crate::traits::error::RaffleError;
use crate::traits::error::RaffleError::*;
use crate::traits::Number;
use openbrush::traits::Storage;

#[derive(Default, Debug)]
#[openbrush::storage_item]
pub struct Data {
    config: Option<Config>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Config {
    pub nb_numbers: u8,
    pub min_number: Number,
    pub max_number: Number,
}

#[openbrush::trait_definition]
pub trait RaffleConfig: Storage<Data> {
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
    fn ensure_config(&self) -> Result<Config, RaffleError> {
        match self.data::<Data>().config {
            None => Err(ConfigNotSet),
            Some(config) => Ok(config),
        }
    }

    /// check if the config is the same as the one given in parameter
    fn ensure_same_config(&self, config: &Config) -> Result<(), RaffleError> {
        // get the correct results for the given raffle
        let this_config = self.ensure_config()?;

        if this_config.nb_numbers != config.nb_numbers
            || this_config.min_number != config.min_number
            || this_config.max_number != config.max_number
        {
            return Err(DifferentConfig);
        }

        Ok(())
    }

    /// check if the numbers respect the config
    fn check_numbers(&mut self, numbers: &[Number]) -> Result<(), RaffleError> {
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
