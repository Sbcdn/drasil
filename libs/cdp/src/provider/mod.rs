pub(crate) mod error;
use crate::modules::txprocessor::common::TransactionUnspentOutputs;
use cardano_serialization_lib::address::Address;
use error::DataProviderError;

use super::models::{
    CardanoNativeAsset, DelegationView, EpochStakeView, HoldingWalletView, TokenInfoView, Tokens,
};

pub trait CardanoDataProvider {
    fn get_wallet_utxos(stake_addr: &str) -> Result<TransactionUnspentOutputs, DataProviderError>;
    fn get_script_utxos(addr: &str) -> Result<TransactionUnspentOutputs, DataProviderError>;
    fn get_asset_utxos_on_addr(addr: &str) -> Result<TransactionUnspentOutputs, DataProviderError>;
    fn get_mint_metadata() -> Result<TokenInfoView, DataProviderError>;
    fn get_first_transaction_from_stake_addr(
        stake_address_in: &str,
    ) -> Result<Address, DataProviderError>;
    fn get_utxo_tokens<T>(utxo_id: T) -> Result<Vec<CardanoNativeAsset>, DataProviderError>;
    fn get_slot() -> Result<i64, DataProviderError>;
    fn get_tot_stake_per_pool(
        pool: &str,
        epoch: i32,
    ) -> Result<Vec<EpochStakeView>, DataProviderError>;
    fn get_deligations_per_pool_for_epochs(
        pool: &str,
        start_epoch: i64,
        end_epoch: i64,
    ) -> Result<Vec<DelegationView>, DataProviderError>;
    fn get_pool_total_stake(pool: &str, epoch: i32) -> Result<i64, DataProviderError>;
    fn get_epoch() -> Result<i32, DataProviderError>;
    fn get_fingerprint(policy: &str, tokenname: &str) -> Result<String, DataProviderError>;
    fn get_token_info(fingerprint_in: &str) -> Result<TokenInfoView, DataProviderError>;
    fn get_stake_registration(stake_addr_in: &str) -> Result<Tokens, DataProviderError>;
    fn get_stake_deregistration(stake_addr_in: &str) -> Result<Tokens, DataProviderError>;
    fn check_stakeaddr_registered(stake_addr_in: &str) -> Result<bool, DataProviderError>;
    fn lookup_token_holders(
        fingerprint_in: &str,
        min_amount: Option<&i64>,
    ) -> Result<Vec<HoldingWalletView>, DataProviderError>;
    fn lookup_nft_token_holders(policy: &str) -> Result<Vec<HoldingWalletView>, DataProviderError>;
    fn pool_valid(pool_id: &str) -> Result<bool, DataProviderError>;
    fn txhash_is_spent(txhash: &str) -> Result<bool, DataProviderError>;
}

pub struct DataProvider<T: CardanoDataProvider> {
    pub provider: T,
}
