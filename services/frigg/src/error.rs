/*
#################################################################################
# Business Source License           See LICENSE.md for full license information.#
# Licensor:             Drasil Blockchain Association                           #
# Licensed Work:        Drasil Application Framework v.0.2. The Licensed Work   #
#                       is © 2022 Drasil Blockchain Association                 #
# Additional Use Grant: You may use the Licensed Work when your application     #
#                       using the Licensed Work is generating less than         #
#                       $150,000 and the entity operating the application       #
#                       engaged equal or less than 10 people.                   #
# Change Date:          Drasil Application Framework v.0.2, change date is two  #
#                       and a half years from release date.                     #
# Change License:       Version 2 or later of the GNU General Public License as #
#                       published by the Free Software Foundation.              #
#################################################################################
*/

use murin::MurinError;
use serde::Serialize;
use sleipnir::SleipnirError;
use std::convert::Infallible;
use thiserror::Error;
use warp::{http::StatusCode, Rejection, Reply};

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("wrong credentials")]
    WrongCredentialsError,
    #[error("jwt token not valid")]
    JWTTokenError,
    #[error("jwt token creation error")]
    JWTTokenCreationError,
    #[error("no auth header")]
    NoAuthHeaderError,
    #[error("invalid auth header")]
    InvalidAuthHeaderError,
    #[error("no permission")]
    NoPermissionError,
    #[error("Email is not verified, please verify your e-mail Address")]
    EmailNotVerified,
    #[error("internal error: {:?}", self)]
    Custom(String),
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

impl warp::reject::Reject for Error {}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::WrongCredentialsError => (StatusCode::FORBIDDEN, e.to_string()),
            Error::NoPermissionError => (StatusCode::UNAUTHORIZED, e.to_string()),
            Error::JWTTokenError => (StatusCode::UNAUTHORIZED, e.to_string()),
            Error::JWTTokenCreationError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            ),
            _ => (StatusCode::BAD_REQUEST, e.to_string()),
        }
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (
            StatusCode::METHOD_NOT_ALLOWED,
            "Method Not Allowed".to_string(),
        )
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    let json = warp::reply::json(&ErrorResponse {
        status: code.to_string(),
        message,
    });

    Ok(warp::reply::with_status(json, code))
}

impl From<MurinError> for Error {
    fn from(err: MurinError) -> Self {
        Error::Custom(err.to_string())
    }
}

impl From<SleipnirError> for Error {
    fn from(err: SleipnirError) -> Self {
        Error::Custom(err.to_string())
    }
}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Self {
        Error::Custom(err)
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Error::Custom(err.to_string())
    }
}
impl From<core::num::ParseIntError> for Error {
    fn from(err: core::num::ParseIntError) -> Self {
        Error::Custom(err.to_string())
    }
}
