use openbrush::contracts::access_control::AccessControlError;

#[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RaffleError {
    AccessControlError(AccessControlError),
    IncorrectRaffle,
    IncorrectStatus,
    IncorrectConfig,
    ConfigNotSet,
    DifferentConfig,
    IncorrectNbNumbers,
    IncorrectNumbers,
    DifferentResults,
    ExistingResults,
    ExistingWinners,
    TransferError,
    AddOverFlow,
    SubOverFlow,
    DivByZero,
    NoReward,
}

/// convertor from AccessControlError to RaffleError
impl From<AccessControlError> for RaffleError {
    fn from(error: AccessControlError) -> Self {
        RaffleError::AccessControlError(error)
    }
}
