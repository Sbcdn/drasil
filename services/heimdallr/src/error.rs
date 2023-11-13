//! Error types used inside the Heimdallr module. All of these error types are triggered by the clien't failure to authenticate.
//! The error type gives a hint as to what caused the authentication to fail. 

use serde::Serialize;
use thiserror::Error;

/// Error types used inside the Heimdallr module. 
#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    /// This is the error for when the HTTP request to Heimdallr is missing an authentication token
    #[error("no auth header")]
    NoAuthHeaderError,
    /// This is the error for when the authentication token in the HTTP request to Heimdallr isn't a bearer token
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    /// This is a miscellaneous error type for cases where no other error type is suitable. 
    #[error("internal error: {:?}", self)]
    Custom(String),
    /// This is the error for when the JWT token wasn't properly configured on the Heimdallr server to begin with 
    /// (i.e. JWT public key is missing)
    #[error("{0}")]
    ImproperlyConfigError(String),
    /// This is the error for when the HTTP request's bearer token's payload's "sub" (subject) field couldn't be 
    /// parsed into an integer (this "sub" field must be an integer) 
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    /// This is the error for when the JWT bearer token (in the HTTP request to Heimdallr server) was incorrect. 
    #[error(transparent)]
    JWTTokenError(#[from] jsonwebtoken::errors::Error),
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

impl warp::reject::Reject for Error {}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Self {
        Error::Custom(err)
    }
}
