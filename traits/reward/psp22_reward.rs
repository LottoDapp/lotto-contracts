use ink_prelude::vec::Vec;
use openbrush::contracts::access_control::AccessControlError;
use openbrush::traits::AccountId;
use openbrush::traits::Balance;

#[openbrush::wrapper]
pub type Psp22RewardRef = dyn Psp22Reward;

#[openbrush::trait_definition]
pub trait Psp22Reward {

    /// Set the rate sharing by teh winners
    /// First winner will receive (total_rewards * ratio[0]) / sum(ratio)
    /// Second winner will receive (total_rewards * ratio[1]) / sum(ratio)
    /// if ratio[n] equals to zero or is empty, tne winner n will receive nothing
    fn _set_ratio_distribution(&mut self, ratio: Vec<Balance>) -> Result<(), RewardError>;

    /// Set the total rewards shared by all winners for a given era
    #[ink(message)]
    fn set_total_rewards(&mut self, era: u128, amount: Balance) -> Result<(), RewardError> ;
    
    /// add the accounts in the list of winners for the given era
    fn _add_winners(&mut self, era: u128, accounts: &Vec<AccountId>) -> Result<PendingReward, RewardError>;

    /// return the pending rewards for a given era and a given account.
    /// If the era is None, the function returns the pending rewards for all era
    /// If the account is None, the function returns the pending rewards for all accounts
    #[ink(message)]
    fn list_pending_rewards_from(&mut self, era: Option<u128>, account: Option<AccountId>) -> Result<Vec<(AccountId, u128, Balance)>, RewardError>;

    /// Return true if the the given account has pending rewards
    #[ink(message)]
    fn has_pending_rewards(&mut self) -> Result<bool, RewardError> ;

    fn _has_pending_rewards_from(&mut self, era: Option<u128>, from: Option<AccountId> ) -> Result<bool, RewardError> ;

    /// claim all pending rewards
    /// After claiming, there is not anymore pending rewards for this account
    #[ink(message)]
    fn claim(&mut self) -> Result<Balance, RewardError> ;

    fn _claim_from(&mut self, from: AccountId) -> Result<Balance, RewardError> ;

}

#[openbrush::trait_definition]
pub trait Internal {
    fn _emit_reward_claimed_event(&self, account: AccountId, amount: Balance);
}


pub struct PendingReward {
    pub era: u128,
    pub given_reward: Balance,
    pub nb_winners: u8
}

#[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RewardError {
    NoReward,
    NoRatioSet,
    TransferError,
    DivByZero,
    MulOverFlow,
    AddOverFlow,
    AccessControlError(AccessControlError)
}

/// convertor from AccessControlError to RewardError
impl From<AccessControlError> for RewardError {
    fn from(error: AccessControlError) -> Self {
        RewardError::AccessControlError(error)
    }
}


