#[macro_use]
extern crate diesel;
pub mod protocol;

pub use crate::protocol::cmd::*;
pub use crate::protocol::connection::*;
pub use crate::protocol::frame::*;
use crate::protocol::parse::*;
pub use crate::protocol::shutdown::*;

pub mod admin;
pub mod authentication;
pub mod encryption;

pub mod database;
pub use database::*;
use drasil_murin::MurinError;

pub mod datamodel;
pub use crate::datamodel::models::*;

pub mod client;
pub use crate::client::Client;

pub mod schema;
pub use schema::*;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = MurinError;