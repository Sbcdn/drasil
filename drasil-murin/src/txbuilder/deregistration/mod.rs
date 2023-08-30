pub mod build_dereg;
pub use build_dereg::{AtDeregBuilder, AtDeregParams};

use cardano_serialization_lib::crypto as ccrypto;
  
use crate::MurinError;
  
#[derive(Debug, Clone)]
pub struct DeregTxData {
    poolhash: String,
    poolkeyhash: ccrypto::Ed25519KeyHash,
    registered: Option<bool>,
}
  
impl DeregTxData {
    pub fn new(poolhash: &str) -> Result<DeregTxData, MurinError> {
        let pool_keyhash = ccrypto::Ed25519KeyHash::from_bech32(poolhash)?;
        Ok(DeregTxData {
            poolhash: poolhash.to_string(),
            poolkeyhash: pool_keyhash,
            registered: None,
        })
    }
   
    pub fn get_poolhash(&self) -> String {
        self.poolhash.clone()
    }
  
    pub fn get_poolkeyhash(&self) -> ccrypto::Ed25519KeyHash {
        self.poolkeyhash.clone()
    }
    
    pub fn get_registered(&self) -> bool {
        if let Some(r) = self.registered {
            r
        } else {
            false
        }
    }

    pub fn set_registered(&mut self, r: Option<bool>) {
        self.registered = r;
    }
}

impl ToString for DeregTxData {
    fn to_string(&self) -> String {
        self.poolhash.clone()
    }
}

impl core::str::FromStr for DeregTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        DeregTxData::new(src)
    }
}

#[cfg(test)]
mod tests {
    use cardano_serialization_lib::crypto::Ed25519KeyHash;
    use core::str::FromStr;

    #[test]
    fn dereg_tx_data() {
        let pool_hash = "pool162ezmfwy0r5px0mll0lkxyshqfh58em6jutl3wasvrnx7w74gcd";
        let mut dereg_tx_data = super::DeregTxData::new(pool_hash).unwrap();

        // initial values
        let get_poolhash = dereg_tx_data.get_poolhash();
        let get_poolkeyhash = dereg_tx_data.get_poolkeyhash();
        let get_registered = dereg_tx_data.get_registered();

        assert_eq!(get_poolhash, pool_hash);
        assert_eq!(get_poolkeyhash, Ed25519KeyHash::from_bech32(pool_hash).unwrap());
        assert_eq!(get_registered, false);

        // set values
        dereg_tx_data.set_registered(Some(true));
        let get_registered_true = dereg_tx_data.get_registered();
        assert_eq!(get_registered_true, true);

        dereg_tx_data.set_registered(Some(false));
        let get_registered_false = dereg_tx_data.get_registered();
        assert_eq!(get_registered_false, false);

        dereg_tx_data.set_registered(None);
        let get_registered_none = dereg_tx_data.get_registered();
        assert_eq!(get_registered_none, false);

        // trait impls
        let to_string = dereg_tx_data.to_string();
        assert_eq!(to_string, pool_hash.to_string());

        let from_str = super::DeregTxData::from_str(pool_hash).unwrap();
        assert_eq!(from_str.poolhash, dereg_tx_data.poolhash);
        assert_eq!(from_str.poolkeyhash, dereg_tx_data.poolkeyhash);
        assert_eq!(from_str.registered, dereg_tx_data.registered);
    }
}