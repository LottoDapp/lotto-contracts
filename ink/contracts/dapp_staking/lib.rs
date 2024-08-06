#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod lotto_dapp_staking {

    type AccountId20 = [u8; 20];

    /// Event emitted when ownership is transferred
    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        contract: AccountId,
        previous: Option<AccountId>,
        new: Option<AccountId>,
    }

    #[ink(event)]
    pub struct ILoveAstar {
        #[ink(topic)]
        sender: AccountId,
    }

    #[ink(event)]
    pub struct ILoveLucky {
        #[ink(topic)]
        sender: AccountId,
    }

    /// Errors occurred in the contract
    #[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        CallerIsNotOwner,
        NewOwnerIsNotSet,
    }

    /// Contract storage
    #[ink(storage)]
    #[derive(Default)]
    pub struct Contract {
        pub owner: Option<AccountId>,
        pub substrate_address: Option<AccountId>,
        pub zk_evm_address: Option<AccountId20>,
    }

    impl Contract {
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut instance = Self::default();
            let caller = instance.env().caller();
            // set the owner of this contract
            instance.inner_set_ownership(Some(caller));
            instance
        }

        #[ink(message)]
        pub fn owner(&self) -> Option<AccountId> {
            self.owner
        }

        #[ink(message)]
        pub fn renounce_ownership(&mut self) -> Result<(), ContractError> {
            // check caller is the owner
            self.ensure_owner()?;
            // remove owner
            self.inner_set_ownership(None);
            Ok(())
        }

        #[ink(message)]
        pub fn transfer_ownership(
            &mut self,
            new_owner: Option<AccountId>,
        ) -> Result<(), ContractError> {
            // check caller is the owner
            self.ensure_owner()?;
            // check the new owner is set
            if new_owner.is_none() {
                return Err(ContractError::NewOwnerIsNotSet);
            }
            // set the new owner
            self.inner_set_ownership(new_owner);
            Ok(())
        }

        fn ensure_owner(&self) -> Result<(), ContractError> {
            if self.owner != Some(self.env().caller()) {
                return Err(ContractError::CallerIsNotOwner);
            }
            Ok(())
        }

        fn inner_set_ownership(&mut self, new_owner: Option<AccountId>) {
            let old_owner = self.owner;
            self.owner = new_owner;
            // emit an event
            self.env().emit_event(OwnershipTransferred {
                contract: self.env().account_id(),
                previous: old_owner,
                new: new_owner,
            });
        }

        #[ink(message)]
        pub fn set_code(&mut self, code_hash: Hash) -> Result<(), ContractError> {
            // check caller is the owner
            self.ensure_owner()?;

            self.env().set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });

            Ok(())
        }

        #[ink(message)]
        pub fn set_substrate_address(&mut self, address: AccountId) -> Result<(), ContractError> {
            // check caller is the owner
            self.ensure_owner()?;
            // set the address
            self.substrate_address = Some(address);
            Ok(())
        }

        #[ink(message)]
        pub fn get_substrate_address(&mut self) -> Option<AccountId> {
            self.substrate_address
        }

        #[ink(message)]
        pub fn lotto_is_deployed_on_astar_substrate(&mut self) -> bool {
            self.substrate_address.is_some()
        }

        #[ink(message)]
        pub fn set_zk_evm_address(&mut self, address: AccountId20) -> Result<(), ContractError> {
            // check caller is the owner
            self.ensure_owner()?;
            // set the address
            self.zk_evm_address = Some(address);
            Ok(())
        }

        #[ink(message)]
        pub fn get_zk_evm_address(&mut self) -> Option<AccountId20> {
            self.zk_evm_address
        }

        #[ink(message)]
        pub fn lotto_is_deployed_on_astar_zk_evm(&mut self) -> bool {
            self.zk_evm_address.is_some()
        }

        #[ink(message)]
        pub fn send_love_to_astar(&mut self) {
            self.env().emit_event(ILoveAstar {
                sender: self.env().caller(),
            });
        }

        #[ink(message)]
        pub fn send_love_to_lucky(&mut self) {
            self.env().emit_event(ILoveLucky {
                sender: self.env().caller(),
            });
        }
    }
}
