#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable, AccessControl, Upgradeable)]
#[openbrush::contract]
pub mod lotto_contract {

    use ink::codegen::{EmitEvent, Env};
    use ink::env::call::{ExecutionInput, Selector};
    use ink::prelude::vec::Vec;
    use openbrush::contracts::access_control::*;
    use openbrush::contracts::ownable::*;
    use openbrush::{modifiers, traits::Storage};

    use lotto::traits::{
        error::*, config, config::*, raffle, raffle::*, LOTTO_MANAGER_ROLE,
    };

    use lotto::traits::config::Config;

    use ::lotto::traits::error::RaffleError;
    use ::lotto::traits::raffle::Raffle;


    use phat_rollup_anchor_ink::traits::{
        js_rollup_anchor, js_rollup_anchor::*, meta_transaction, meta_transaction::*,
        rollup_anchor, rollup_anchor::*,
    };

    /// Event emitted when the participant is registered
    #[ink(event)]
    pub struct ParticipationRegistered {
        #[ink(topic)]
        num_raffle: u32,
        #[ink(topic)]
        participant: AccountId,
        numbers: Vec<u8>,
    }

    /// Event emitted when the raffle is started
    #[ink(event)]
    pub struct RaffleStarted {
        #[ink(topic)]
        num_raffle: u32,
    }

    /// Event emitted when the raffle is ended
    #[ink(event)]
    pub struct RaffleEnded {
        #[ink(topic)]
        num_raffle: u32,
    }

    /// Event emitted when the raffle result is received
    #[ink(event)]
    pub struct ResultReceived {
        #[ink(topic)]
        num_raffle: u32,
        numbers: Vec<u8>,
    }

    /// Event emitted when the winners are revealed
    #[ink(event)]
    pub struct WinnersRevealed {
        #[ink(topic)]
        num_raffle: u32,
        winners: Vec<AccountId>,
    }

    /// Event emitted when an error is received
    #[ink(event)]
    pub struct ErrorReceived {
        #[ink(topic)]
        num_raffle: u32,
        /// error
        error: Vec<u8>,
    }

    /// Errors occurred in the contract
    #[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        AccessControlError(AccessControlError),
        RaffleError(RaffleError),
    }

    /// convertor from AccessControlError to ContractError
    impl From<AccessControlError> for ContractError {
        fn from(error: AccessControlError) -> Self {
            ContractError::AccessControlError(error)
        }
    }

    /// convertor from RaffleError to ContractError
    impl From<RaffleError> for ContractError {
        fn from(error: RaffleError) -> Self {
            ContractError::RaffleError(error)
        }
    }

    /// convertor from RaffleError to ContractError
    impl From<ContractError> for RollupAnchorError {
        fn from(error: ContractError) -> Self {
            ink::env::debug_println!("Error: {:?}", error);
            RollupAnchorError::UnsupportedAction
        }
    }

    /// Contract storage
    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct Contract {
        #[storage_field]
        ownable: ownable::Data,
        #[storage_field]
        access: access_control::Data,
        #[storage_field]
        rollup_anchor: rollup_anchor::Data,
        #[storage_field]
        meta_transaction: meta_transaction::Data,
        #[storage_field]
        js_rollup_anchor: js_rollup_anchor::Data,
        #[storage_field]
        config: config::Data,
        #[storage_field]
        lotto: raffle::Data,
    }

    impl RaffleConfig for Contract {}
    impl Raffle for Contract {}
    impl RollupAnchor for Contract {}
    impl MetaTransaction for Contract {}
    impl JsRollupAnchor for Contract {}

    impl Contract {
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut instance = Self::default();
            let caller = instance.env().caller();
            // set the owner of this contract
            ownable::Internal::_init_with_owner(&mut instance, caller);
            // set the admin of this contract
            access_control::Internal::_init_with_admin(&mut instance, Some(caller));
            // grant the role manager
            AccessControl::grant_role(&mut instance, LOTTO_MANAGER_ROLE, Some(caller))
                .expect("Should grant the role LOTTO_MANAGER_ROLE");
            instance
        }

        #[ink(message)]
        #[modifiers(only_role(DEFAULT_ADMIN_ROLE))]
        pub fn terminate_me(&mut self) -> Result<(), ContractError> {
            self.env().terminate_contract(self.env().caller());
        }

        #[ink(message)]
        pub fn get_manager_role(&self) -> RoleType {
            LOTTO_MANAGER_ROLE
        }
    }

    #[derive(scale::Encode, scale::Decode)]
    pub struct RaffleRequestMessage {
        pub era: u32,
        pub nb_winners: u16,
        pub excluded: Vec<AccountId>,
    }

    #[derive(scale::Encode, scale::Decode)]
    pub struct RaffleResponseMessage {
        pub era: u32,
        pub skipped: bool,
        pub rewards: Balance,
        pub winners: Vec<AccountId>,
    }

    impl rollup_anchor::MessageHandler for Contract {
        fn on_message_received(&mut self, action: Vec<u8>) -> Result<(), RollupAnchorError> {
            let response = JsRollupAnchor::on_message_received::<
                RaffleRequestMessage,
                RaffleResponseMessage,
            >(self, action)?;
            match response {
                MessageReceived::Ok { output } => {
                    // register the info
                    //self.save_response(&output)?;
                }
                MessageReceived::Error { error, input } => {
                    // we received an error
                    /*
                    let timestamp = self.env().block_timestamp();
                    self.env().emit_event(ErrorReceived {
                        era: input.era,
                        error,
                        timestamp,
                    });

                     */
                }
            }

            Ok(())
        }
    }

    impl raffle::Internal for Contract {

        fn emit_participation(&self, num_raffle: u32, participant: AccountId, numbers: Vec<u8>){

        }
        fn emit_results(&self, num_raffle: u32, result: Vec<u8>){

        }
        fn emit_winners(&self, num_raffle: u32, winners: Vec<AccountId>){

        }
    }


    impl rollup_anchor::EventBroadcaster for Contract {
        fn emit_event_message_queued(&self, _id: u32, _data: Vec<u8>) {
            // no queue here
        }
        fn emit_event_message_processed_to(&self, _id: u32) {
            // no queue here
        }
    }

    impl meta_transaction::EventBroadcaster for Contract {
        fn emit_event_meta_tx_decoded(&self) {
            // do nothing
        }
    }
}
