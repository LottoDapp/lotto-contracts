#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

#[cfg(test)]
#[openbrush::contract]
pub mod raffle {
    use ink_storage::traits::SpreadAllocate;
    use lucky::impls::{
        *,
        oracle::*,
        reward::psp22_reward,
        reward::psp22_reward::*,
        raffle::*,
    };
    use openbrush::contracts::access_control::{*, access_control};
    use openbrush::traits::Storage;

    #[ink(storage)]
    #[derive(Default, Storage, SpreadAllocate)]
    pub struct Contract {
        #[storage_field]
        oracle_data: oracle::Data,
        #[storage_field]
        reward: psp22_reward::Data,
        #[storage_field]
        raffle: raffle::Data,
        #[storage_field]
        access: access_control::Data,
    }

    impl Raffle for Contract{}
    impl AccessControl for Contract{}

    impl Contract {
        #[ink(constructor)]
        pub fn default() -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut Self| {
                instance.oracle_data = oracle::Data::default();
                instance.reward = psp22_reward::Data::default();
                instance.raffle = raffle::Data::default();
                let caller = instance.env().caller();
                instance._init_with_admin(caller);
                instance.grant_role(ORACLE_DATA_MANAGER, caller).expect("Should grant the role ORACLE_DATA_MANAGER");
                instance.grant_role(RAFFLE_MANAGER, caller).expect("Should grant the role RAFFLE_MANAGER");
                instance.grant_role(REWARD_MANAGER, caller).expect("Should grant the role REWARD_MANAGER");
                instance.grant_role(REWARD_VIEWER, caller).expect("Should grant the role REWARD_VIEWER");
            })
        }

        #[ink(message)]
        pub fn run_raffle(&mut self, era: u32) -> Result<(), ContractError> {

            // get the oracle data
            let oracle_data = self.get_data(era);

            let participants = oracle_data.participants;
            let rewards = oracle_data.rewards;

            // select the participants
            let winners = self._run_raffle(era, participants, rewards)?;

            // transfer the rewards and the winners
            ink_env::pay_with_call!(self.fund_rewards_and_add_winners(era, winners), rewards)?;

            Ok(())
        }
    }

    /// Errors occurred in the contract
    #[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        RaffleError(RaffleError),
        RewardError(RewardError)
    }


    /// convertor from RaffleError to ContractError
    impl From<RaffleError> for ContractError {
        fn from(error: RaffleError) -> Self {
            ContractError::RaffleError(error)
        }
    }

    /// convertor from RewardError to ContractError
    impl From<RewardError> for ContractError {
        fn from(error: RewardError) -> Self {
            ContractError::RewardError(error)
        }
    }    
    


    impl psp22_reward::Internal for Contract {
        fn _emit_rewards_claimed_event(&self, _account: AccountId, _amount: Balance){
            // no event for the tests
        }
        fn _emit_pending_reward_event(&self, _account: AccountId, _era: u32, _amount: Balance){
            // no event for the tests
        }
    }

    mod tests {
        use ink_env::debug_println;
        use ink_lang as ink;
        use openbrush::test_utils::accounts;

        use super::*;


        #[ink::test]
        fn test_ratio_distribution() {
            let mut contract = super::Contract::default();

            // 50 + 30 + 20 > 80 => Error
            let result = contract.set_ratio_distribution(vec![50, 30, 20], 90);
            match result {
                Err(IncorrectRatio) => debug_println!("Incorrect Ratio as expected"),
                _ => panic!("Error 1"),
            };

            // 50 + 30 + 20 = 100 => Ok
            let result = contract.set_ratio_distribution(vec![50, 30, 20], 100);
            match result {
                Ok(_) => debug_println!("No Error as expected"),
                _ => panic!("Error 2"),
            };

            // 50 + 30 + 20 < 150 => Ok
            let result = contract.set_ratio_distribution(vec![50, 30, 20], 150);
            match result {
                Ok(_) => debug_println!("No Error as expected"),
                _ => panic!("Error 2"),
            };

        }

        #[ink::test]
        fn test_run_raffle_no_ratio_set() {
            let mut contract = super::Contract::default();

            //contract.set_ratio_distribution(vec![50, 30, 20], 100).unwrap();

            let accounts = accounts();
            let participants = vec![
                (accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)
                ];

            let result = contract._run_raffle(1, participants, 1000);
            match result {
                Err(NoRatioSet) => debug_println!("NoRatioSet as expected"),
                _ => panic!("Error 1"),
            };
        }

        #[ink::test]
        fn test_run_raffle_no_participant() {
            let mut contract = super::Contract::default();

            contract.set_ratio_distribution(vec![50, 30, 20], 100).unwrap();

            let participants = vec![];

            let result = contract._run_raffle(1, participants, 1000);
            match result {
                Err(NoParticipant) => debug_println!("NoParticipant as expected"),
                _ => panic!("Error 1"),
            };
        }


        #[ink::test]
        fn test_run_raffle_no_reward() {
            let mut contract = super::Contract::default();

            contract.set_ratio_distribution(vec![50, 30, 20], 100).unwrap();

            let accounts = accounts();
            let participants = vec![
                (accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)
                ];

            let result = contract._run_raffle(1, participants, 0);
            match result {
                Err(RaffleError::NoReward) => debug_println!("NoParticipant as expected"),
                _ => panic!("Error 1"),
            };
        }
   
        #[ink::test]
        fn test_run_raffle_with_zero_in_ratio() {
            let mut contract = super::Contract::default();
            let accounts = accounts();

            let participants = vec![
                (accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)
                ];

            // second winner receive nada
            contract.set_ratio_distribution(vec![50, 0, 50], 100).unwrap();

            // select the participants
            let winners = contract._run_raffle(1, participants, 1000).unwrap();

            // assert two differents winners
            assert_eq!(winners.len(), 2); 
            assert!(winners[0] != winners[1]); 

            let mut total_rewards = 0;
            for (_, r) in  winners {
                total_rewards += r;
            }
            // assert all rewards are given
            assert_eq!(total_rewards, 1000); 
        }

        #[ink::test]
        fn test_run_raffle_not_already_done() {

            let mut contract = super::Contract::default();
            
            contract.set_ratio_distribution(vec![100], 100).unwrap();

            let accounts = accounts();
            let rewards = 1000;

            // first raffle => success
            let participants = vec![(accounts.alice, 100000)];
            contract._run_raffle(2, participants, rewards).unwrap();

            // second raffle for the same era => failure
            let participants = vec![(accounts.alice, 100000)];
            let result = contract._run_raffle(2, participants, rewards);
            match result {
                Err(RaffleError::RaffleAlreadyDone) => debug_println!("RaffleAlreadyDone as expected"),
                _ => panic!("Error 1"),
            };

            // second raffle for previous era => failure
            let participants = vec![(accounts.alice, 100000)];
            let result = contract._run_raffle(1, participants, rewards);
            match result {
                Err(RaffleError::RaffleAlreadyDone) => debug_println!("RaffleAlreadyDone as expected"),
                _ => panic!("Error 2"),
            };

            // raffle for next era => success
            let participants = vec![(accounts.alice, 100000)];
            contract._run_raffle(3, participants, rewards).unwrap();

        }


        #[ink::test]
        fn test_run_raffle_share_full_rewards() {
            let mut contract = super::Contract::default();
            let accounts = accounts();

            let rewards = 1000;

            let participants = vec![
                (accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)
                ];

            contract.set_ratio_distribution(vec![50, 30, 20], 100).unwrap();

            // select the participants
            let winners = contract._run_raffle(1, participants, rewards).unwrap();

            // assert three differents winners
            assert_eq!(winners.len(), 3); 
            assert!(winners[0] != winners[1]); 
            assert!(winners[0] != winners[2]); 
            assert!(winners[1] != winners[2]); 

            let mut total_rewards = 0;
            for (_, r) in  winners {
                total_rewards += r;
            }
            // assert all rewards are given
            assert_eq!(total_rewards, 1000); 
        }


        #[ink::test]
        fn test_run_raffle_share_partial_rewards() {
            let mut contract = super::Contract::default();
            let accounts = accounts();

            let participants = vec![
                (accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)
                ];

            contract.set_ratio_distribution(vec![50, 30, 20], 200).unwrap();

            // select the participants
            let winners = contract._run_raffle(1, participants, 1000).unwrap();

            // assert three differents winners
            assert_eq!(winners.len(), 3); 
            assert!(winners[0] != winners[1]); 
            assert!(winners[0] != winners[2]); 
            assert!(winners[1] != winners[2]); 

            let mut total_rewards = 0;
            for (_, r) in  winners {
                total_rewards += r;
            }
            // expected rewards: (50 + 30 + 20) / 200 * 1000 = 500
            assert_eq!(total_rewards, 500); 
        }



        #[ink::test]
        fn test_raffle_contract()  {

            let mut contract = super::Contract::default();
            contract.set_ratio_distribution(vec![50, 30, 20], 100).unwrap();

            let era = 1;
            let accounts = accounts();

            contract.add_participants(
                era, 
                vec![(accounts.alice, 100000), (accounts.bob, 100000), (accounts.charlie, 100000), 
                (accounts.django, 100000), (accounts.eve, 100000), (accounts.frank, 100000)]
            ).unwrap();

            contract.set_rewards(era, 1000).unwrap();

            contract.run_raffle(era).unwrap();

            let mut nb_winners = 0;
            let mut total_rewards = 0;
            if let Some(r) = get_reward(&mut contract, accounts.bob) {
                nb_winners += 1;
                total_rewards += r;
            }
            if let Some(r) = get_reward(&mut contract, accounts.alice) {
                nb_winners += 1;
                total_rewards += r;
            }
            if let Some(r) = get_reward(&mut contract, accounts.charlie) {
                nb_winners += 1;
                total_rewards += r;
            }
            if let Some(r) = get_reward(&mut contract, accounts.django) {
                nb_winners += 1;
                total_rewards += r;
            }
            if let Some(r) = get_reward(&mut contract, accounts.eve) {
                nb_winners += 1;
                total_rewards += r;
            }
            if let Some(r) = get_reward(&mut contract, accounts.frank) {
                nb_winners += 1;
                total_rewards += r;
            }            
            // assert three differents winners
            assert_eq!(nb_winners, 3); 
            // assert three differents winners
            assert_eq!(total_rewards, 1000); 

        }

        pub fn get_reward(contract: &mut super::Contract, account: AccountId) -> Option<u128> {

            if contract._has_pending_rewards_from(account) {
                let pending_rewards = contract.get_pending_rewards_from(account).unwrap();
                debug_println!("Account {:?} has pending rewards: {:?} ", account, pending_rewards);
                return pending_rewards;
            }
            None

        }

    }
}