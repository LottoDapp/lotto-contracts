#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable, AccessControl, Upgradeable)]
#[openbrush::contract]
pub mod lotto_contract {

    use ink::codegen::{EmitEvent, Env};
    use ink::prelude::vec::Vec;
    use openbrush::contracts::access_control::*;
    use openbrush::contracts::ownable::*;
    use openbrush::{modifiers, traits::Storage};

    use lotto::traits::{
        config, config::*, error::*, raffle, raffle::*, Number, LOTTO_MANAGER_ROLE,
    };

    use phat_rollup_anchor_ink::traits::{
        meta_transaction, meta_transaction::*, rollup_anchor, rollup_anchor::*,
    };

    /// Event emitted when the participant is registered
    #[ink(event)]
    pub struct ParticipationRegistered {
        #[ink(topic)]
        num_raffle: u32,
        #[ink(topic)]
        participant: AccountId,
        numbers: Vec<Number>,
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
        numbers: Vec<Number>,
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
    /*
       /// convertor from RaffleError to RollupAnchorError
       impl From<error::RaffleError> for rollup_anchor::RollupAnchorError {
           fn from(error: error::RaffleError) -> Self {
               ink::env::debug_println!("Error: {:?}", error);
               RollupAnchorError::UnsupportedAction
           }
       }
    */

    /// Message to request the lotto lotto_draw or the list of winners
    /// message pushed in the queue by the Ink! smart contract and read by the offchain rollup
    #[derive(Eq, PartialEq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct LottoRequestMessage {
        /// id of the requestor
        requestor_id: AccountId,
        /// lotto_draw number
        draw_num: u32,
        /// request
        request: Request,
    }

    #[derive(Eq, PartialEq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Request {
        /// request to lotto_draw the n number between min and max values
        /// arg1: number of numbers for the lotto_draw
        /// arg2:  smallest number for the lotto_draw
        /// arg2:  biggest number for the lotto_draw
        DrawNumbers(u8, Number, Number),
        /// request to check if there is a winner for the given numbers
        CheckWinners(Vec<Number>),
    }

    /// Message sent to provide the lotto lotto_draw or the list of winners
    /// response pushed in the queue by the offchain rollup and read by the Ink! smart contract
    #[derive(scale::Encode, scale::Decode)]
    struct LottoResponseMessage {
        /// initial request
        request: LottoRequestMessage,
        /// response
        response: Response,
    }

    #[derive(Eq, PartialEq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Response {
        /// list of numbers
        Numbers(Vec<Number>),
        /// list of winners
        Winners(Vec<AccountId>),
        /// when an error occurs
        Error(Vec<u8>),
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
        config: config::Data,
        #[storage_field]
        lotto: raffle::Data,
    }

    impl RaffleConfig for Contract {}
    impl Raffle for Contract {}

    impl RollupAnchor for Contract {}
    impl MetaTransaction for Contract {}

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

        #[ink(message)]
        pub fn participate(&mut self, numbers: Vec<Number>) -> Result<(), ContractError> {
            // check if the numbers are correct
            self.check_numbers(&numbers)?;
            // register the participation
            self.inner_participate(numbers)?;

            Ok(())
        }

        ///
        /// ONLY FOR TEST
        ///
        #[ink(message)]
        pub fn test_set_results(
            &mut self,
            num_raffle: u32,
            numbers: Vec<Number>,
        ) -> Result<(), ContractError> {
            // check if the numbers are correct
            self.check_numbers(&numbers)?;
            // set the results
            self.inner_set_results(num_raffle, numbers)?;
            Ok(())
        }

        ///
        /// ONLY FOR TEST
        ///
        #[ink(message)]
        pub fn test_set_winners(
            &mut self,
            num_raffle: u32,
            winners: Vec<AccountId>,
        ) -> Result<(), ContractError> {
            self.inner_set_winners(num_raffle, winners)?;
            Ok(())
        }
    }

    impl rollup_anchor::MessageHandler for Contract {
        fn on_message_received(&mut self, action: Vec<u8>) -> Result<(), RollupAnchorError> {
            // parse the response
            let message: LottoResponseMessage = scale::Decode::decode(&mut &action[..])
                .or(Err(RollupAnchorError::FailedToDecode))?;

            let num_raffle = message.request.draw_num;

            match message.response {
                Response::Numbers(numbers) => self
                    .inner_set_results(num_raffle, numbers)
                    .or(Err(RollupAnchorError::UnsupportedAction))?,
                Response::Winners(winners) => self
                    .inner_set_winners(num_raffle, winners)
                    .or(Err(RollupAnchorError::UnsupportedAction))?,
                Response::Error(error) => {
                    self.env().emit_event(ErrorReceived { num_raffle, error })
                }
            }

            Ok(())
        }
    }

    impl raffle::Internal for Contract {
        fn emit_participation_registered(
            &self,
            num_raffle: u32,
            participant: AccountId,
            numbers: Vec<Number>,
        ) {
            // emit the event
            self.env().emit_event(ParticipationRegistered {
                num_raffle,
                participant,
                numbers,
            });
        }

        fn emit_results(&self, num_raffle: u32, numbers: Vec<Number>) {
            // emit the event
            self.env().emit_event(ResultReceived {
                num_raffle,
                numbers,
            });
        }

        fn emit_winners(&self, num_raffle: u32, winners: Vec<AccountId>) {
            // emit the event
            self.env().emit_event(WinnersRevealed {
                num_raffle,
                winners,
            });
        }
    }

    /// Event emitted when a message is pushed in the queue
    #[ink(event)]
    pub struct MessageQueued {
        id: u32,
        data: Vec<u8>,
    }

    /// Event emitted when a message is processed
    #[ink(event)]
    pub struct MessageProcessedTo {
        id: u32,
    }

    impl rollup_anchor::EventBroadcaster for Contract {
        fn emit_event_message_queued(&self, id: u32, data: Vec<u8>) {
            self.env().emit_event(MessageQueued { id, data });
        }
        fn emit_event_message_processed_to(&self, id: u32) {
            self.env().emit_event(MessageProcessedTo { id });
        }
    }

    /// Event emitted when a Meta Tx is decoded
    #[ink(event)]
    pub struct MetaTxDecoded {}

    impl meta_transaction::EventBroadcaster for Contract {
        fn emit_event_meta_tx_decoded(&self) {
            self.env().emit_event(MetaTxDecoded {});
        }
    }
}
