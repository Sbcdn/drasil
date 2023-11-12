/*
use super::RegistrationDatum;
use crate::{
    models::{
        BuilderResult, EarthNodeConfig, RegisterEarthNode, TransactionSchema, UnsignedTransaction,
    },
    modules::txprocessor::{
        error::TransactionBuildingError, transactions::smartcontract::enregistration::ENREGCONTRACT,
    },
};
use cardano_serialization_lib::{
    address::{Address, EnterpriseAddress, StakeCredential},
    crypto::ScriptHash,
    fees::LinearFee,
    plutus::{self, ExUnitPrices, PlutusData, PlutusScript},
    tx_builder::{
        tx_inputs_builder::TxInputsBuilder, CoinSelectionStrategyCIP2, TransactionBuilder,
        TransactionBuilderConfigBuilder,
    },
    utils::{hash_plutus_data, min_ada_for_output, to_bignum, Value},
    AssetName, Assets, MultiAsset, TransactionOutput, UnitInterval,
};
use cdp::provider::CardanoDataProvider;
use dcslc::{
    calc_min_ada_for_utxo, decode_transaction_unspent_outputs, extract_assets, get_pubkeyhash,
    get_stakeaddr_from_addr, make_fingerprint, Token, TransactionUnspentOutput,
};
use log::debug;
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ENRegMetadata {
    operator_address: String,
    validator_address: String,
    moniker: String,
    ennft: String,
}


pub(crate) async fn handle_register_earth_node(
    tx_schema: TransactionSchema,
) -> Result<BuilderResult, TransactionBuildingError> {
let op_data = match tx_schema.operation {
    Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
    None => return Err(TransactionBuildingError::StandardTransactionBuildingError),
};

let policy =
    std::env::var("ENNFT_POLICY").expect("No ENNFT policyID set for this tx-building service");

let smartcontract = PlutusScript::from_bytes_v2(hex::decode(ENREGCONTRACT.as_str())?)?;
let scripthash = smartcontract.hash();
let cred = StakeCredential::from_scripthash(&scripthash);

// check against wallet
let utxos = decode_transaction_unspent_outputs(
    &tx_schema.utxos.unwrap(),
    tx_schema.collateral.as_ref(),
    tx_schema.excludes.as_ref(),
)?;

let ennft_utxos = utxos.find_utxo_containing_policy(&policy)?;

let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
for utxo in ennft_utxos {
    let multiassets = extract_assets(&utxo, &policy)?;
    assets.push((multiassets, utxo));
}

debug!("Assets:\n{:?}", assets);

let mut ennfts_cip30 = Vec::<(Address, Token)>::new();

for asset in assets {
    let sh = ScriptHash::from_bytes(hex::decode(&policy)?)?;
    let assets = asset.0.get(&sh).unwrap();
    let asset_names = assets.keys();
    for i in 0..assets.len() {
        let an = asset_names.get(i);
        let amt = assets.get(&an).unwrap();
        ennfts_cip30.push((asset.1.output().address(), (sh.clone(), an, amt)));
    }
}

// check against dataprovider
let dp = cdp::DataProvider::new(cdp::DBSyncProvider::new(cdp::Config {
    db_path: std::env::var("DBSYNC_URL").unwrap(),
}));

let stake_address = if let Some(stake_addr) = tx_schema.stake_address {
    dcslc::addr_from_str(&stake_addr[0].clone())?
        .to_bech32(None)
        .unwrap()
} else {
    return Err(TransactionBuildingError::RewardAddressNotFound);
};
debug!("Stake Address: {}", stake_address);
let utxos_dp = dp.wallet_utxos(&stake_address).await.unwrap();
debug!("\n\nWallet UTxOs empty: {:?}\n", &utxos_dp);
let first_address = dp.first_transaction_from_stake_addr(&stake_address).await?;
let script_address = EnterpriseAddress::new(first_address.network_id()?, &cred).to_address();
log::debug!("\nScript Address: {}\n", script_address.to_bech32(None)?);

let pubkeyhash = get_pubkeyhash(&first_address)?;

let ennft_utxo = utxos_dp.find_utxo_containing_policy(&policy)?;

if ennft_utxo.is_empty() {
    return Err(TransactionBuildingError::Custom(
        "wallet does not contain any ENNFTs, registration not possible without ENNFT"
            .to_owned(),
    ));
}

let mut ennfts = Vec::<(Address, Token)>::new();

for utxo in ennft_utxo.clone() {
    let ma = utxo.output().amount().multiasset().unwrap();
    let sh = ScriptHash::from_bytes(hex::decode(&policy)?)?;
    let assets = ma.get(&sh).unwrap();
    let asset_names = assets.keys();
    for i in 0..assets.len() {
        let an = asset_names.get(i);
        let amt = assets.get(&an).unwrap();
        ennfts.push((utxo.output().address(), (sh.clone(), an, amt)));
    }
}
debug!("ENNFTS:\n{:?}", &ennfts);
//ToDo: assert_eq fails sometimes due to ordering, build a test function which checks on equal content
//assert_eq!(ennfts, ennfts_cip30);

let token_info = dp.token_info(&op_data.ennft_assetname).await?;
assert_eq!(token_info.policy, policy);

let mut valid_ennfts = ennfts.clone();
valid_ennfts = valid_ennfts
    .iter()
    .filter(|n| n.1 .1 == AssetName::new(hex::decode(&token_info.tokenname).unwrap()).unwrap())
    .map(|n| n.to_owned())
    .collect();

if valid_ennfts.is_empty() {
    return Err(TransactionBuildingError::Custom(
        "wallet does not contain valid ENNFTs, please speicfy which ENNFT to use if you have several".to_owned(),
    ));
}

// search input utxo
let input_utxo: Vec<_> = ennft_utxo
    .filter(|n| {
        n.output()
            .amount()
            .multiasset()
            .unwrap()
            .get_asset(&valid_ennfts[0].1 .0, &valid_ennfts[0].1 .1)
            .compare(&to_bignum(1))
            == 0
    })
    .collect();

if input_utxo.len() != 1 {
    return Err(TransactionBuildingError::Custom(
        "could not select input".to_owned(),
    ));
}
let input_utxo = input_utxo[0].clone();

// Create specific config hash
let ennft_fingerprint = make_fingerprint(
    &valid_ennfts[0].1 .0.to_hex(),
    &hex::encode(valid_ennfts[0].1 .1.name()),
)?;

//let stake_address = get_stakeaddr_from_addr(&valid_ennfts[0].0)?;

// Registration Datum
let regdat = RegistrationDatum {
    enOperatorAddress: op_data.config.operator_address.as_bytes().to_vec(),
    enConsensusPubkey: op_data.config.consensus_pub_key.as_bytes().to_vec(),
    enMerkleTreeRoot: op_data.config.merkle_tree_root.as_bytes().to_vec(),
    enCceAddress: op_data.config.cce_address.as_bytes().to_vec(),
    enUsedNftTn: valid_ennfts[0].1 .1.clone(),
    enOwner: pubkeyhash.clone(),
};

Err(())

}
 */