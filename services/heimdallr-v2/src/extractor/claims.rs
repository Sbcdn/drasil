//! Claim extractor
//!
//! This module implements the claim extractor from a bearer token.
use axum::extract::{FromRef, FromRequestParts};
use axum::headers::authorization::{Authorization, Bearer};
use axum::http::request::Parts;
use axum::{async_trait, RequestPartsExt, TypedHeader};
use jsonwebtoken::{decode, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AuthError;
use crate::state::AppState;

/// Claims represents the claims data from JWT
#[derive(Debug, Deserialize, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct Claims {
    sub: String,
    exp: usize,
}

impl Claims {
    /// Return the customer ID in this claim.
    pub fn get_customer_id(&self) -> Result<u64, AuthError> {
        self.sub.parse().map_err(|_| AuthError::WrongCredentials)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingCredentials)?;

        let app_state = AppState::from_ref(state);

        // Decode the user data
        let token_data = decode::<Claims>(
            bearer.token(),
            &app_state.jwt_decoding_key,
            &Validation::default(),
        )
        .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}
