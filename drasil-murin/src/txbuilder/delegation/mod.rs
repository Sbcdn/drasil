pub mod build_deleg;
pub use build_deleg::{AtDelegBuilder, AtDelegParams};

use cardano_serialization_lib::crypto as ccrypto;

use crate::MurinError;

#[derive(Debug, Clone)]
pub struct DelegTxData {
    poolhash: String,
    poolkeyhash: ccrypto::Ed25519KeyHash,
    registered: Option<bool>,
}

impl DelegTxData {
    pub fn new(poolhash: &str) -> Result<DelegTxData, MurinError> {
        let pool_keyhash = ccrypto::Ed25519KeyHash::from_bech32(poolhash)?;
        Ok(DelegTxData {
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

impl ToString for DelegTxData {
    fn to_string(&self) -> String {
        self.poolhash.clone()
    }
}

impl core::str::FromStr for DelegTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        DelegTxData::new(src)
    }
}

#[cfg(test)]
mod tests {
    use cardano_serialization_lib::crypto::Ed25519KeyHash;
    use core::str::FromStr;

    use crate::MurinError;

    #[test]
    fn deleg_tx_data() -> Result<(), MurinError>{
        let pool_hash = "pool162ezmfwy0r5px0mll0lkxyshqfh58em6jutl3wasvrnx7w74gcd";
        let mut deleg_tx_data = super::DelegTxData::new(pool_hash)?;

        // initial values
        let get_poolhash = deleg_tx_data.get_poolhash();
        let get_poolkeyhash = deleg_tx_data.get_poolkeyhash();
        let get_registered = deleg_tx_data.get_registered();

        assert_eq!(get_poolhash, pool_hash);
        assert_eq!(get_poolkeyhash, Ed25519KeyHash::from_bech32(pool_hash)?);
        assert_eq!(get_registered, false);

        // set values
        deleg_tx_data.set_registered(Some(true));
        let get_registered_true = deleg_tx_data.get_registered();
        assert_eq!(get_registered_true, true);

        deleg_tx_data.set_registered(Some(false));
        let get_registered_false = deleg_tx_data.get_registered();
        assert_eq!(get_registered_false, false);

        deleg_tx_data.set_registered(None);
        let get_registered_none = deleg_tx_data.get_registered();
        assert_eq!(get_registered_none, false);

        // trait impls
        let to_string = deleg_tx_data.to_string();
        assert_eq!(to_string, pool_hash.to_string());

        let from_str = super::DelegTxData::from_str(pool_hash)?;
        assert_eq!(from_str.poolhash, deleg_tx_data.poolhash);
        assert_eq!(from_str.poolkeyhash, deleg_tx_data.poolkeyhash);
        assert_eq!(from_str.registered, deleg_tx_data.registered);

        Ok(())
    }
}
