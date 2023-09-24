use crate::clib;
use thiserror::Error;
use std::string::String;
use clib::error::{JsError, DeserializeError};

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

impl From<String> for TxToolsError {
    fn from(err: String) -> Self {
        TxToolsError::Custom(err)
    }
}

impl From<JsError> for TxToolsError {
    fn from(err: JsError) -> Self {
        TxToolsError::Custom(err.to_string())
    }
}

impl From<DeserializeError> for TxToolsError {
    fn from(err: DeserializeError) -> Self {
        TxToolsError::Custom(err.to_string())
    }
}
