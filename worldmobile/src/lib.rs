//! # Worldmobile
//!
//! This crate contains various smart contracts.
#![forbid(unsafe_code, clippy::unwrap_used)]
#![warn(
    missing_debug_implementations,
    // missing_docs,
    nonstandard_style,
    // clippy::missing_docs_in_private_items
)]

pub mod config;
pub mod error;
pub mod models;
pub mod registration;
