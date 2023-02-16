/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
pub mod administration;
pub mod airdrops;
pub mod apiauth;
pub mod contracts;
pub mod discounts;
pub mod jobs;
pub mod minting;
pub mod rewards;
pub mod user;
pub mod whitelist;

pub use minting::*;

pub mod error;
pub use error::*;

pub use gungnir::WhitelistType;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
