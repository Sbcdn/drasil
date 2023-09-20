use crate::clib;
use thiserror::Error;

#[allow(clippy::enum_variant_names, dead_code)]
#[derive(Error, Debug)]
pub enum TxToolsError {
    #[error("one of the provided inputs is empty")]
    EmptyInputs,
    #[error("For the given address no transfer-wallet was registered")]
    NoWalletForAddress,
    #[error("For the given address there was more than one transfer-wallet registered")]
    TooManyWalletsForAddress,
    #[error("No payment value set for source")]
    SourceNoPaymentValueSet,
    #[error("Custom Error")]
    Custom(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    MurinError(#[from] crate::error::MurinError),
    #[error(transparent)]
    VarError(#[from] std::env::VarError),
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error(transparent)]
    UTF8Error(#[from] std::string::FromUtf8Error),
}

impl From<std::string::String> for TxToolsError {
    fn from(err: std::string::String) -> Self {
        TxToolsError::Custom(err)
    }
}

impl From<clib::error::JsError> for TxToolsError {
    fn from(err: clib::error::JsError) -> Self {
        TxToolsError::Custom(err.to_string())
    }
}

impl From<clib::error::DeserializeError> for TxToolsError {
    fn from(err: clib::error::DeserializeError) -> Self {
        TxToolsError::Custom(err.to_string())
    }
}
