/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
extern crate diesel;
pub mod chelper;
pub mod cip30;
pub mod cost;
pub mod error;
pub mod txbuilders;
pub mod utxomngr;

pub use chelper::*;
pub use cip30::*;
pub use error::MurinError;
pub use txbuilders::*;

pub use cardano_serialization_lib as clib;
pub use clib::*;
pub use utxomngr::*;

pub use cryptoxide::*;

extern crate dotenv;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

macro_rules! pub_struct {
    ($name:ident {$($field:ident: $t:ty,)*}) => {
        #[derive(Serialize,Deserialize,Debug, Clone, PartialEq)] // ewww
        pub struct $name {
            $(pub $field: $t),*
        }
    }
}
