//! This module defines the error type.

use murin::clib::error::{DeserializeError, JsError};

/// Error type see module level [self](documentation)
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// This is murin related errors
    #[error("wrong credentials")]
    JwtError,
    #[error("jwt token creation error")]
    NoAuthHeaderError,
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("wrong transaction pattern")]
    TxSchemaError,
    #[error("could not resolve standtand transaction")]
    StandardTransactionError,
    #[error("no operation parameter provided")]
    NoOperation,
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    MurinError(#[from] murin::MurinError),
    #[error("error building standard transaction")]
    StandardTransactionBuildingError,
    #[error("error building smart contract transaction")]
    SmartContractBuildingError,
    #[error("error building native transaction")]
    NativeScriptBuildingError,
    #[error("error deserializing cardano address from string")]
    AddressDeserializationError,
    #[error(transparent)]
    HexDecoderError(#[from] hex::FromHexError),
    #[error("unable to determine reward-address from address")]
    RewardAddressNotFound,
    #[error(transparent)]
    DataProviderError(#[from] cdp::DataProviderError),
    #[error("internal error: {0}")]
    Custom(String),
}

/// The `Result` type is an alias to `std::result::Result`
/// with `self::Error` as error.
pub type Result<T> = std::result::Result<T, Error>;

impl From<JsError> for Error {
    fn from(err: JsError) -> Self {
        Error::from(murin::MurinError::from(err))
    }
}

impl From<DeserializeError> for Error {
    fn from(err: DeserializeError) -> Self {
        Error::from(murin::MurinError::from(err))
    }
}
