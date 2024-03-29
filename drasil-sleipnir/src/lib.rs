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

pub use drasil_gungnir::WhitelistType;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
