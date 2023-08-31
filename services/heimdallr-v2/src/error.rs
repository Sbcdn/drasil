//! This module defines the error types.

use std::fmt;
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
}

/// Authentication errors type.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// This is the error when the credentials do not match.
    WrongCredentials,

    /// This is the error when the credential is missing.
    MissingCredentials,

    /// This is the error when the credential is invalid.
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "missing credentials"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::InvalidToken => "invalid token",
            Self::MissingCredentials => "missing token",
            Self::WrongCredentials => "wrong credential",
        };
        write!(f, "{s}")
    }
}
