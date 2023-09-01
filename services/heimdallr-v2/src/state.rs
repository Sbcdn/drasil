//! # AppState
//!
//! This module defines the application state type.

use jsonwebtoken::DecodingKey;
use secrecy::{ExposeSecret, Secret};

use crate::error::{Error, Result};

/// Application state type.
#[derive(Clone)]
pub struct AppState {
    /// JWT decoding key.
    pub jwt_decoding_key: DecodingKey,
    /// Odin connection URL
    pub odin_url: String,
}

impl AppState {
    /// Create new application state.
    pub fn new(jwt: Secret<String>, odin_url: String) -> Result<Self> {
        let jwt_decoding_key =
            DecodingKey::from_ec_pem(jwt.expose_secret().as_bytes()).map_err(Error::JwtError)?;

        let state = Self {
            jwt_decoding_key,
            odin_url,
        };

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;

    #[test]
    fn state() {
        let secret = concat!(
            "-----BEGIN PUBLIC KEY-----\n",
            "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEMMkapa1mVNQtUdWP9B61OpMcuBHmw+",
            "LwS66RkRJ3gYlrXCisZwWaNQo3nkNjRujIVVI9jEGCWYRdECga9lUjrg=\n",
            "-----END PUBLIC KEY-----",
        );

        let secret = Secret::new(secret.into());
        let state = AppState::new(secret, "odin_url".into());
        assert!(state.is_ok())
    }
}
