/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use serde::Serialize;
use thiserror::Error;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("internal error: {:?}", self)]
    Custom(String),
    #[error("rmq error: {0}")]
    RMQError(#[from] lapin::Error),
    #[error("rmq pool error: {0}")]
    RMQPoolError(#[from] deadpool_lapin::PoolError),
    #[error("Utf8Error")]
    UTF8Error(#[from] std::str::Utf8Error),
    #[error("JsonError")]
    JsonError(#[from] serde_json::Error),
    #[error("MintAPIError: {0}")]
    MintAPIError(#[from] gungnir::RWDError),
    #[error("SCLError: {0}")]
    CSLError(#[from] murin::clib::error::JsError),
    #[error("MurinError: {0}")]
    MurinError(#[from] murin::MurinError),
    #[error("MimirError: {0}")]
    MimirError(#[from] mimir::MimirError),
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Self {
        Error::Custom(err)
    }
}
