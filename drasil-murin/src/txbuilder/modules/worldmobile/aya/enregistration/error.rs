use std::convert::Infallible;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum TransactionBuildingError {
    #[error("internal error: {:?}", self)]
    Custom(String),
    #[error("error building standard transaction")]
    StandardTransactionBuildingError,
    #[error("error building smart contract transaction")]
    SmartContractBuildingError,
    #[error("error building native transaction")]
    NativeScriptBuildingError,
    #[error("error decerializing cardano address from string")]
    AddressDecerializationError,
    #[error(transparent)]
    HexDecoderError(#[from] hex::FromHexError),
    #[error("error coudl not determine reward-address from address")]
    RewardAddressNotFound,
    #[error(transparent)]
    JSONError(#[from] serde_json::Error),
}

impl From<std::string::String> for TransactionBuildingError {
    fn from(err: std::string::String) -> Self {
        TransactionBuildingError::Custom(err)
    }
}

impl From<cardano_serialization_lib::error::DeserializeError> for TransactionBuildingError {
    fn from(err: cardano_serialization_lib::error::DeserializeError) -> Self {
        TransactionBuildingError::Custom(err.to_string())
    }
}

impl From<cardano_serialization_lib::error::JsError> for TransactionBuildingError {
    fn from(err: cardano_serialization_lib::error::JsError) -> Self {
        TransactionBuildingError::Custom(err.to_string())
    }
}
