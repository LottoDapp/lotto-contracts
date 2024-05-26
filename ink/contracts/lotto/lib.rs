#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(Ownable, AccessControl, Upgradeable)]
#[openbrush::contract]
pub mod lotto_contract {
    use ink::codegen::{EmitEvent, Env};
    use ink::prelude::vec::Vec;
    use lotto::traits::{
        config, config::*, error::*, raffle, raffle::*, reward, reward::*, Number, RaffleId,
        LOTTO_MANAGER_ROLE,
    };
    use openbrush::contracts::access_control::*;
    use openbrush::contracts::ownable::*;
    use openbrush::{modifiers, traits::Storage};
    use phat_rollup_anchor_ink::traits::{
        meta_transaction, meta_transaction::*, rollup_anchor, rollup_anchor::*,
    };
    use scale::Encode;

    /// Event emitted when the participant is registered
    #[ink(event)]
    pub struct ParticipationRegistered {
        #[ink(topic)]
        raffle_id: RaffleId,
        #[ink(topic)]
        participant: AccountId,
        numbers: Vec<Number>,
    }

    /// Event emitted when the raffle is started
    #[ink(event)]
    pub struct RaffleStarted {
        #[ink(topic)]
        raffle_id: RaffleId,
    }

    /// Event emitted when the raffle is ended
    #[ink(event)]
    pub struct RaffleEnded {
        #[ink(topic)]
        raffle_id: RaffleId,
    }

    /// Event emitted when the raffle result is received
    #[ink(event)]
    pub struct ResultReceived {
        #[ink(topic)]
        raffle_id: RaffleId,
        numbers: Vec<Number>,
    }

    /// Event emitted when the winners are revealed
    #[ink(event)]
    pub struct WinnersRevealed {
        #[ink(topic)]
        raffle_id: RaffleId,
        winners: Vec<AccountId>,
    }

    /// Event emitted when a reward is pending
    #[ink(event)]
    pub struct PendingReward {
        #[ink(topic)]
        account: AccountId,
        amount: Balance,
    }

    /// Event emitted when a user claim rewards
    #[ink(event)]
    pub struct RewardsClaimed {
        #[ink(topic)]
        account: AccountId,
        amount: Balance,
    }

    /// Errors occurred in the contract
    #[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        AccessControlError(AccessControlError),
        RaffleError(RaffleError),
        RollupAnchorError(RollupAnchorError),
        TransferError,
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
    impl From<RollupAnchorError> for ContractError {
        fn from(error: RollupAnchorError) -> Self {
            ContractError::RollupAnchorError(error)
        }
    }

    /// convertor from RaffleError to ContractError
    impl From<ContractError> for RollupAnchorError {
        fn from(error: ContractError) -> Self {
            ink::env::debug_println!("Error: {:?}", error);
            RollupAnchorError::UnsupportedAction
        }
    }

    /// Message to request the lotto lotto_draw or the list of winners
    /// message pushed in the queue by the Ink! smart contract and read by the offchain rollup
    #[derive(Eq, PartialEq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct LottoRequestMessage {
        /// raffle id
        pub raffle_id: RaffleId,
        /// request
        pub request: Request,
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
    pub struct LottoResponseMessage {
        /// initial request
        pub request: LottoRequestMessage,
        /// response
        pub response: Response,
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
        #[storage_field]
        reward: reward::Data,
    }

    impl RaffleConfig for Contract {}
    impl Raffle for Contract {}
    impl RewardManager for Contract {}

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
        pub fn participate(&mut self, numbers: Vec<Number>) -> Result<(), ContractError> {
            // check if the numbers are correct
            RaffleConfig::check_numbers(self, &numbers)?;
            // check if the user can participate (raffle is open)
            Raffle::can_participate(self)?;
            // save the participation with an event
            let participant = Self::env().caller();
            let raffle_id = Raffle::get_current_raffle_id(self);
            self.env().emit_event(ParticipationRegistered {
                raffle_id,
                participant,
                numbers,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn participate_batch(
            &mut self,
            numbers: Vec<Vec<Number>>,
        ) -> Result<(), ContractError> {
            // check if the numbers are correct
            for n in numbers {
                self.participate(n)?;
            }

            Ok(())
        }

        #[ink(message)]
        #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
        pub fn set_config(&mut self, config: Config) -> Result<(), RaffleError> {
            // check the status, we can set the config only when the raffle is not started yet
            let status = Raffle::get_current_status(self);
            if status != Status::NotStarted {
                return Err(RaffleError::IncorrectStatus);
            }

            // update the config
            RaffleConfig::set_config(self, config)?;

            Ok(())
        }

        #[ink(message)]
        #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
        pub fn start_raffle(&mut self) -> Result<RaffleId, ContractError> {
            let raffle_id = self.inner_start_raffle()?;
            Ok(raffle_id)
        }

        fn inner_start_raffle(&mut self) -> Result<RaffleId, ContractError> {
            // start new raffle
            let raffle_id = Raffle::start_new_raffle(self)?;

            // emit the event
            self.env().emit_event(RaffleStarted { raffle_id });

            Ok(raffle_id)
        }

        #[ink(message)]
        #[openbrush::modifiers(access_control::only_role(LOTTO_MANAGER_ROLE))]
        pub fn complete_raffle(&mut self) -> Result<(), ContractError> {
            // stop the current raffle
            Raffle::stop_current_raffle(self)?;

            // emit the event
            let raffle_id = Raffle::get_current_raffle_id(self);
            self.env().emit_event(RaffleEnded { raffle_id });

            // request the draw numbers
            let config = RaffleConfig::ensure_config(self)?;
            let message = LottoRequestMessage {
                raffle_id,
                request: Request::DrawNumbers(
                    config.nb_numbers,
                    config.min_number,
                    config.max_number,
                ),
            };
            RollupAnchor::push_message(self, &message)?;

            Ok(())
        }

        fn inner_set_results(
            &mut self,
            raffle_id: RaffleId,
            config: Config,
            numbers: Vec<Number>,
        ) -> Result<(), ContractError> {
            // check if the config used to select the number is correct
            RaffleConfig::ensure_same_config(self, &config)?;

            // check if the numbers are correct
            RaffleConfig::check_numbers(self, &numbers)?;

            // set the result
            Raffle::set_results(self, raffle_id, numbers.clone())?;

            // save in the kv store the last raffle id used for verification
            const LAST_RAFFLE: u32 = ink::selector_id!("LAST_RAFFLE_FOR_VERIF");
            RollupAnchor::set_value(self, &LAST_RAFFLE.encode(), Some(&raffle_id.encode()));

            // emmit the event
            self.env().emit_event(ResultReceived {
                raffle_id,
                numbers: numbers.clone(),
            });

            // request to check the winners
            let message = LottoRequestMessage {
                raffle_id,
                request: Request::CheckWinners(numbers),
            };
            self.push_message(&message)?;

            Ok(())
        }

        pub fn inner_set_winners(
            &mut self,
            raffle_id: RaffleId,
            numbers: Vec<Number>,
            winners: Vec<AccountId>,
        ) -> Result<(), ContractError> {
            // check if the winners were selected based on the correct numbers
            Raffle::ensure_same_results(self, raffle_id, &numbers)?;

            // set the winners in the raffle
            Raffle::set_winners(self, raffle_id, winners.clone())?;

            // emmit the event
            self.env().emit_event(WinnersRevealed {
                raffle_id,
                winners: winners.clone(),
            });

            // set the winners in the reward manager
            if !winners.is_empty() {
                RewardManager::add_winners(self, winners)?;
            } else {
                // start automatically the new raffle if there is no winner
                self.inner_start_raffle()?;
            }

            Ok(())
        }

        #[ink(message)]
        #[modifiers(only_role(DEFAULT_ADMIN_ROLE))]
        pub fn register_attestor(
            &mut self,
            account_id: AccountId,
        ) -> Result<(), AccessControlError> {
            AccessControl::grant_role(self, ATTESTOR_ROLE, Some(account_id))?;
            Ok(())
        }

        #[ink(message)]
        pub fn get_attestor_role(&self) -> RoleType {
            ATTESTOR_ROLE
        }

        #[ink(message)]
        pub fn get_manager_role(&self) -> RoleType {
            LOTTO_MANAGER_ROLE
        }

        #[ink(message)]
        #[modifiers(only_role(DEFAULT_ADMIN_ROLE))]
        pub fn terminate_me(&mut self) -> Result<(), ContractError> {
            self.env().terminate_contract(self.env().caller());
        }

        #[ink(message)]
        #[openbrush::modifiers(only_role(DEFAULT_ADMIN_ROLE))]
        pub fn withdraw(&mut self, value: Balance) -> Result<(), ContractError> {
            let caller = Self::env().caller();
            self.env()
                .transfer(caller, value)
                .map_err(|_| ContractError::TransferError)?;
            Ok(())
        }
    }

    impl rollup_anchor::MessageHandler for Contract {
        fn on_message_received(&mut self, action: Vec<u8>) -> Result<(), RollupAnchorError> {
            // parse the response
            let message: LottoResponseMessage = scale::Decode::decode(&mut &action[..])
                .or(Err(RollupAnchorError::FailedToDecode))?;

            let raffle_id = message.request.raffle_id;

            match message.response {
                Response::Numbers(numbers) => {
                    let config = match message.request.request {
                        Request::DrawNumbers(nb_numbers, min_number, max_number) => Config {
                            nb_numbers,
                            min_number,
                            max_number,
                        },
                        _ => return Err(RollupAnchorError::UnsupportedAction),
                    };
                    self.inner_set_results(raffle_id, config, numbers)
                        .or(Err(RollupAnchorError::UnsupportedAction))?
                }
                Response::Winners(winners) => {
                    let numbers = match message.request.request {
                        Request::CheckWinners(numbers) => numbers,
                        _ => return Err(RollupAnchorError::UnsupportedAction),
                    };
                    self.inner_set_winners(raffle_id, numbers, winners)
                        .or(Err(RollupAnchorError::UnsupportedAction))?
                }
            }

            Ok(())
        }
    }

    /// Event emitted when a message is pushed in the queue
    #[ink(event)]
    pub struct MessageQueued {
        #[ink(topic)]
        id: u32,
        data: Vec<u8>,
    }

    /// Event emitted when a message is processed
    #[ink(event)]
    pub struct MessageProcessedTo {
        #[ink(topic)]
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

    impl meta_transaction::EventBroadcaster for Contract {
        fn emit_event_meta_tx_decoded(&self) {
            // do nothing
        }
    }

    impl reward::Internal for Contract {
        fn emit_pending_reward_event(&self, account: AccountId, amount: Balance) {
            self.env().emit_event(PendingReward { account, amount });
        }

        fn emit_rewards_claimed_event(&self, account: AccountId, amount: Balance) {
            self.env().emit_event(RewardsClaimed { account, amount });
        }
    }
}
