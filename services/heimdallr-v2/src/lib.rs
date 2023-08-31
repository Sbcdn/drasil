//! # Heimdallr
//!
//! Heimadallr is an HTTP gateway interface to Odin.

#![forbid(unsafe_code, clippy::unwrap_used)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    clippy::missing_docs_in_private_items,
    clippy::missing_const_for_fn
)]

pub mod bootstrap;
pub mod error;
pub mod settings;
