use rweb::warp::{http::StatusCode, Rejection, Reply};
use std::convert::Infallible;
use thiserror::Error;

use crate::{models::ErrorResponse, modules::txprocessor::error::TransactionBuildingError};

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum DataProviderError {
    #[error("internal error: {:?}", self)]
    Custom(String),
    #[error("error in data provider")]
    General,
    #[error("error in data provider using dbsync")]
    DBsyncError,
    #[error("error in data provider using blockfrost")]
    BlockFrostError,
    #[error("error in data provider using koios")]
    KoiosError,
    #[error(transparent)]
    HexDecoderError(#[from] hex::FromHexError),
    #[error(transparent)]
    JSONError(#[from] serde_json::Error),
}

impl rweb::warp::reject::Reject for DataProviderError {}

impl From<std::string::String> for DataProviderError {
    fn from(err: std::string::String) -> Self {
        DataProviderError::Custom(err)
    }
}

impl From<jsonwebtoken::errors::Error> for DataProviderError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        DataProviderError::Custom(err.to_string())
    }
}

impl From<cardano_serialization_lib::error::DeserializeError> for DataProviderError {
    fn from(err: cardano_serialization_lib::error::DeserializeError) -> Self {
        DataProviderError::Custom(err.to_string())
    }
}

impl From<cardano_serialization_lib::error::JsError> for DataProviderError {
    fn from(err: cardano_serialization_lib::error::JsError) -> Self {
        DataProviderError::Custom(err.to_string())
    }
}
