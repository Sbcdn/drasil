//! This module defines the error types.

use std::io;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror;

/// Result an alias for the `Result` type with `self::Error` as error
pub type Result<T> = std::result::Result<T, Error>;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O related errors
    #[error(transparent)]
    IoError(#[from] io::Error),

    /// Improperly configuration errors
    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),

    /// This is JWT related errors.
    #[error(transparent)]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// Huggin errors
    #[error(transparent)]
    HugginError(#[from] drasil_hugin::Error),

    /// Authenticatication and authorization errors
    #[error(transparent)]
    AuthError(#[from] AuthError),

    /// General transaction errors.
    #[error(transparent)]
    TransactionError(#[from] TransactionError),

    /// General  errors like parsing error or conversion
    /// errors that do not fit in the other variants.
    #[error("{0}")]
    UnexpectedError(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error_message = self.to_string();
        let status = match self {
            Self::IoError(_)
            | Self::ConfigError(_)
            | Self::JwtError(_)
            | Self::HugginError(_)
            | Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::AuthError(err) => return err.into_response(),
            Self::TransactionError(err) => return err.into_response(),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

/// Authentication errors type.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// This is the error when the credentials do not match.
    #[error("wrong credentials")]
    WrongCredentials,

    /// This is the error when the credential is missing.
    #[error("missing credentials")]
    MissingCredentials,

    /// This is the error when the credential is invalid.
    #[error("invalid token")]
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let error_message = self.to_string();
        let status = match self {
            Self::WrongCredentials => StatusCode::UNAUTHORIZED,
            Self::MissingCredentials | Self::InvalidToken => StatusCode::BAD_REQUEST,
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

/// The transaction error type enumerate the various error encountered during
/// transction processing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransactionError {
    /// This is theerror when the transaction is invalid
    #[error("unable to process an invalid transaction.")]
    Invalid,

    /// This is the error when a transaction does not reflect the current system state.
    #[error("the system encountered a conflict while processing this transaction.")]
    Conflict,
}

impl IntoResponse for TransactionError {
    fn into_response(self) -> Response {
        let error_message = self.to_string();
        let status = match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Conflict => StatusCode::CONFLICT,
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
