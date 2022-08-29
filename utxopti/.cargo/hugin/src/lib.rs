/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
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

pub mod datamodel;
pub use crate::datamodel::hephadata::*;

pub mod client;
pub use crate::client::Client;

pub mod schema;
pub use schema::*;

extern crate dotenv;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

//Temporär
pub type Error = Box<dyn std::error::Error + Send + Sync>;
/// A specialized `Result` type for hugin library .
/// This is defined as a convenience.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
