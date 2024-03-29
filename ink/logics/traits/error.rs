use openbrush::contracts::access_control::AccessControlError;

#[derive(Debug, Eq, PartialEq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
pub enum RaffleError {
    IncorrectConfig,
    ConfigNotSet,
    IncorrectNbNumbers,
    IncorrectNumbers,
    AccessControlError(AccessControlError),
    ExistingResults,
    ExistingWinners,
}

/// convertor from AccessControlError to RaffleError
impl From<AccessControlError> for RaffleError {
    fn from(error: AccessControlError) -> Self {
        RaffleError::AccessControlError(error)
    }
}
