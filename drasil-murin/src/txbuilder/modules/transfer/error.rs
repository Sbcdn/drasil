use crate::clib;
use crate::modules::txtools::error::TxToolsError;
use thiserror::Error;
use std::string::String;
use clib::error::{JsError, DeserializeError};

#[allow(clippy::enum_variant_names, dead_code)]
#[derive(Error, Debug)]
pub enum TransferError {
    #[error("For the given source the wrong transfer-wallet was provided")]
    WrongWalletForAddress,
    #[error("For the given address no transfer-wallet was registered")]
    NoWalletForAddress,
    #[error("For the given cid no transfer-wallet was registered")]
    NoWalletForCID,
    #[error("For the given address there was more than one transfer-wallet registered")]
    TooManyWalletsForAddress,
    #[error("No payment value set for source")]
    SourceNoPaymentValueSet,
    #[error("The input and outputs of this transaction do not balance")]
    TxNotBalanced,
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
    #[error(transparent)]
    TxToolsError(#[from] TxToolsError),
}

impl From<String> for TransferError {
    fn from(err: String) -> Self {
        TransferError::Custom(err)
    }
}

impl From<JsError> for TransferError {
    fn from(err: JsError) -> Self {
        TransferError::Custom(err.to_string())
    }
}

impl From<DeserializeError> for TransferError {
    fn from(err: DeserializeError) -> Self {
        TransferError::Custom(err.to_string())
    }
}
