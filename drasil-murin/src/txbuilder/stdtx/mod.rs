use std::{fmt::Display, str::FromStr};

pub mod build_deleg;
pub use build_deleg::{AtDelegBuilder, AtDelegParams};
pub mod build_dereg;
pub use build_dereg::{AtDeregBuilder, AtDeregParams};
pub mod build_reward_withdrawal;
pub use build_reward_withdrawal::{AtAWBuilder, AtAWParams};

use cardano_serialization_lib::{crypto as ccrypto, utils};
use cardano_serialization_lib::{address::Address, utils::BigNum, AssetName, PolicyID};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::MurinError;
pub mod build_cpo;
pub mod build_wallet_asset_transfer;

/// This module incoperates the Types for Standard Transactions
/// We consider all Transactions which do not use smart contracts or native scripts as Standard Transactions.
/// 
/// These Standard Transactions are implemented: 
/// - Build Stakepool Delegation Transaction (build_deleg.rs)
/// 
/// - Build Stakepool Deregistration (build_dereg.rs)
/// 
/// - Build ADA Reward Withdrawal Transaction (build_reward_withdrawal.rs)
/// 
/// - Build Customer Pay Out (will payout from a system managed reward multisig contract to the reward contract owner) (build_cpo.rs);
/// 
/// - Build Wallet Asset Transfer Transaction (build_wallet_asset_transfer.rs)
/// 
 

/// A StdAssetHandle is a handle to an Asset on the Cardano blockchain.
/// It's fields contain all the information which can be related to a Asset / Token on the Cardano blockchain.
/// To specify ADA instead of a native token only the 'amount' field is used and the other fields are 'None'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdAssetHandle {
    pub fingerprint: Option<String>,
    pub policy: Option<PolicyID>,
    pub tokenname: Option<AssetName>,
    pub amount: BigNum,
    pub metadata: Option<serde_json::Value>,
}

/// An AssetTransfer defines the transfer of an arbitrary amount of asset from a source wallet (the request sending wallet) to a 'reiceiver' wallet
/// The to be transfered Tokens are specified as (Std)AssetHandles. The metadata field hold the transaction metadata of the minting transaction if present. 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTransfer {
    pub receiver: Address,
    pub assets: Vec<StdAssetHandle>,
    pub metadata: Option<String>,
}

/// StandardTxData is the 'Wallet Asset Transfer Transaction' specific type and holds the wallet addresses of the source wallet 
/// and the AssetTransfers which are intended to be send by this source wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTxData {
    pub wallet_addresses: Vec<Address>,
    pub transfers: Vec<AssetTransfer>,
}

impl Display for StandardTxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", json!(&self))
    }
}

impl FromStr for StandardTxData {
    type Err = crate::MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str::<StandardTxData>(src)?)
    }
}

/// WithdrawlTxData is the 'ADA Reward Withdrawal Transaction' specific type and holds the amount to be withdrawn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalTxData {
    pub withdrawl: Option<BigNum>,
}

impl WithdrawalTxData {
    pub fn new(withdrawl: u64) -> Result<WithdrawalTxData, MurinError> {
        Ok(WithdrawalTxData{
            withdrawl: Some(utils::to_bignum(withdrawl)),
        })
    }
}

/// DelegTxData is the 'Stakepool Delegation Transaction' specific type and holds:
/// the poolhash in bech32 (String), the public key of the stake pool generated from the poolhash
/// and if the delegator (the sending wallet) is already register with its stake address or not.
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

impl std::str::FromStr for DelegTxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        DelegTxData::new(src)
    }
}

/// DeregTxData is the 'Stakepool Deregistration Transaction' specific type
#[derive(Debug, Clone)]
pub struct DeregTxData {
    // Nonesense ...
    registered: Option<bool>,
}

impl DeregTxData {
    pub fn new() -> Result<DeregTxData, MurinError> {
        Ok(DeregTxData { registered: None })
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

#[cfg(test)]
mod tests {
    use crate::MurinError;
    use cardano_serialization_lib::crypto::Ed25519KeyHash;
    use std::str::FromStr;

    #[test]
    fn deleg_tx_data() -> Result<(), MurinError> {
        let pool_hash = "pool162ezmfwy0r5px0mll0lkxyshqfh58em6jutl3wasvrnx7w74gcd";
        let mut deleg_tx_data = super::DelegTxData::new(pool_hash)?;

        // initial values
        let get_poolhash = deleg_tx_data.get_poolhash();
        let get_poolkeyhash = deleg_tx_data.get_poolkeyhash();
        let get_registered = deleg_tx_data.get_registered();

        assert_eq!(get_poolhash, pool_hash);
        assert_eq!(get_poolkeyhash, Ed25519KeyHash::from_bech32(pool_hash)?);
        assert!(!get_registered);

        // set values
        deleg_tx_data.set_registered(Some(true));
        let get_registered_true = deleg_tx_data.get_registered();
        assert!(get_registered_true);

        deleg_tx_data.set_registered(Some(false));
        let get_registered_false = deleg_tx_data.get_registered();
        assert!(!get_registered_false);

        deleg_tx_data.set_registered(None);
        let get_registered_none = deleg_tx_data.get_registered();
        assert!(!get_registered_none);

        // trait impls
        let to_string = deleg_tx_data.to_string();
        assert_eq!(to_string, pool_hash.to_string());

        let from_str = super::DelegTxData::from_str(pool_hash)?;
        assert_eq!(from_str.poolhash, deleg_tx_data.poolhash);
        assert_eq!(from_str.poolkeyhash, deleg_tx_data.poolkeyhash);
        assert_eq!(from_str.registered, deleg_tx_data.registered);

        Ok(())
    }

    #[test]
    fn dereg_tx_data() {
        let _pool_hash = "pool162ezmfwy0r5px0mll0lkxyshqfh58em6jutl3wasvrnx7w74gcd";
        let mut dereg_tx_data = super::DeregTxData::new().unwrap();

        // initial values
        let get_registered = dereg_tx_data.get_registered();

        assert!(!get_registered);

        // set values
        dereg_tx_data.set_registered(Some(true));
        let get_registered_true = dereg_tx_data.get_registered();
        assert!(get_registered_true);

        dereg_tx_data.set_registered(Some(false));
        let get_registered_false = dereg_tx_data.get_registered();
        assert!(!get_registered_false);

        dereg_tx_data.set_registered(None);
        let get_registered_none = dereg_tx_data.get_registered();
        assert!(!get_registered_none);
    }
}
