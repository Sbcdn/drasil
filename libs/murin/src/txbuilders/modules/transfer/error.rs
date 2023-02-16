/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::clib;
use crate::modules::txtools::error::TxToolsError;
use thiserror::Error;

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
    ParseIntError(#[from] core::num::ParseIntError),
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

impl From<std::string::String> for TransferError {
    fn from(err: std::string::String) -> Self {
        TransferError::Custom(err)
    }
}

impl From<clib::error::JsError> for TransferError {
    fn from(err: clib::error::JsError) -> Self {
        TransferError::Custom(err.to_string())
    }
}

impl From<clib::error::DeserializeError> for TransferError {
    fn from(err: clib::error::DeserializeError) -> Self {
        TransferError::Custom(err.to_string())
    }
}
