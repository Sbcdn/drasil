use cdp::provider::CardanoDataProvider;
use cdp::{DBSyncProvider, DataProvider};
use murin::address::{Address, EnterpriseAddress, StakeCredential};
use murin::crypto::ScriptHash;
use murin::fees::LinearFee;
use murin::plutus::{ConstrPlutusData, ExUnitPrices, PlutusData, PlutusList, PlutusScript};
use murin::tx_builder::tx_inputs_builder::TxInputsBuilder;
use murin::tx_builder::{
    CoinSelectionStrategyCIP2, TransactionBuilder, TransactionBuilderConfigBuilder,
};
use murin::txbuilder::modules::txtools::utxo_handling;
use murin::utils::{to_bignum, Value};
use murin::MurinError;
use murin::{self, cardano, utils, wallet, AssetName, Assets, MultiAsset, UnitInterval};
use murin::{TransactionOutput, TransactionUnspentOutput, TransactionUnspentOutputs};
use serde::{Deserialize, Serialize};

use super::RegistrationDatum;
use crate::config::RegistrationConfig;
use crate::error::{Error, Result};
use crate::models::{
    BuilderResult, RegisterEarthNode, Token, TransactionSchema, UnsignedTransaction,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ENRegMetadata {
    operator_address: String,
    validator_address: String,
    moniker: String,
    ennft: String,
}

pub(crate) async fn register_earth_node(
    config: RegistrationConfig,
    provider: DataProvider<DBSyncProvider>,
    tx_schema: TransactionSchema,
) -> Result<BuilderResult> {
    let op_data = match tx_schema.operation {
        Some(d) => serde_json::from_value::<RegisterEarthNode>(d)?,
        None => return Err(Error::StandardTransactionBuildingError),
    };

    let smartcontract =
        PlutusScript::from_bytes_v2(hex::decode(&config.contract)?).map_err(MurinError::from)?;
    let scripthash = smartcontract.hash();
    let cred = StakeCredential::from_scripthash(&scripthash);

    // check against wallet
    let utxos: TransactionUnspentOutputs = wallet::transaction_unspent_outputs_from_string_vec(
        &tx_schema.utxos.unwrap(),
        tx_schema.collateral.as_ref(),
        tx_schema.excludes.as_ref(),
    )?;

    let ennft_utxos: TransactionUnspentOutputs =
        utxos.find_utxo_containing_policy(&config.policy)?;

    let mut assets = Vec::<(MultiAsset, TransactionUnspentOutput)>::new();
    for utxo in ennft_utxos {
        let multiassets = wallet::extract_assets(&utxo, &config.policy)?;
        assets.push((multiassets, utxo));
    }

    tracing::debug!("Assets:\n{:?}", assets);

    let mut ennfts_cip30 = Vec::<(Address, Token)>::new();

    for asset in assets {
        let sh = ScriptHash::from_bytes(hex::decode(&config.policy)?).map_err(MurinError::from)?;
        let assets = asset.0.get(&sh).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts_cip30.push((asset.1.output().address(), (sh.clone(), an, amt)));
        }
    }

    let stake_address = if let Some(stake_addr) = tx_schema.stake_address {
        wallet::address_from_string_non_async(&stake_addr[0].clone())
            .unwrap()
            .to_bech32(None)
            .unwrap()
    } else {
        return Err(Error::RewardAddressNotFound);
    };
    tracing::debug!("Stake Address: {}", stake_address);
    let utxos = provider.wallet_utxos(&stake_address).await.unwrap();

    tracing::debug!("\n\nWallet UTxOs empty: {:?}\n", &utxos);
    let first_address = provider
        .first_transaction_from_stake_addr(&stake_address)
        .await?;
    let net_id = first_address.network_id().map_err(MurinError::from)?;
    let script_address = EnterpriseAddress::new(net_id, &cred).to_address();
    tracing::debug!(
        "\nScript Address: {}\n",
        script_address.to_bech32(None).map_err(MurinError::from)?
    );

    let pubkeyhash = wallet::payment_keyhash_from_address(&first_address)?;

    let ennft_utxo = utxos
        .find_utxo_containing_policy(&config.policy)
        .map_err(|err| Error::Custom(err.to_string()))?;

    if ennft_utxo.is_empty() {
        return Err(Error::Custom(
            "wallet does not contain any ENNFTs, registration not possible without ENNFT"
                .to_owned(),
        ));
    }

    let mut ennfts = Vec::<(Address, Token)>::new();

    for utxo in ennft_utxo.clone() {
        let ma = utxo.output().amount().multiasset().unwrap();
        let sh = ScriptHash::from_bytes(hex::decode(&config.policy)?).map_err(MurinError::from)?;
        let assets = ma.get(&sh).unwrap();
        let asset_names = assets.keys();
        for i in 0..assets.len() {
            let an = asset_names.get(i);
            let amt = assets.get(&an).unwrap();
            ennfts.push((utxo.output().address(), (sh.clone(), an, amt)));
        }
    }
    tracing::debug!("ENNFTS:\n{:?}", &ennfts);
    //ToDo: assert_eq fails sometimes due to ordering, build a test function which checks on equal content
    //assert_eq!(ennfts, ennfts_cip30);

    let token_info = provider.token_info(&op_data.ennft_assetname).await?;
    assert_eq!(token_info.policy, config.policy);

    let mut valid_ennfts = ennfts.clone();
    valid_ennfts = valid_ennfts
        .iter()
        .filter(|n| n.1 .1 == AssetName::new(hex::decode(&token_info.tokenname).unwrap()).unwrap())
        .map(|n| n.to_owned())
        .collect();

    if valid_ennfts.is_empty() {
        return Err(Error::Custom(
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
                .compare(&utils::to_bignum(1))
                == 0
        })
        .collect();

    if input_utxo.len() != 1 {
        return Err(Error::Custom("could not select input".to_owned()));
    }
    let input_utxo = input_utxo[0].clone();

    // Create specific config hash
    let ennft_fingerprint = cardano::make_fingerprint(
        &valid_ennfts[0].1 .0.to_hex(),
        &hex::encode(valid_ennfts[0].1 .1.name()),
    )?;

    //let stake_address = get_stakeaddr_from_addr(&valid_ennfts[0].0)?;

    // Registration Datum
    let regdat = RegistrationDatum {
        operator_address: op_data.config.operator_address.as_bytes().to_vec(),
        validator_address: op_data.config.validator_address.as_bytes().to_vec(),
        moniker: op_data.config.moniker.as_bytes().to_vec(),
        used_nft: valid_ennfts[0].1 .1.clone(),
        owner: pubkeyhash.clone(),
    };

    //
    // Transaction Building
    //
    let mut builderconfig = TransactionBuilderConfigBuilder::new();
    builderconfig = builderconfig.fee_algo(&LinearFee::new(
        &utils::to_bignum(44),
        &utils::to_bignum(155381),
    ));
    builderconfig = builderconfig.pool_deposit(&utils::to_bignum(500000000));
    builderconfig = builderconfig.key_deposit(&utils::to_bignum(2000000));
    builderconfig = builderconfig.max_value_size(5000);
    builderconfig = builderconfig.max_tx_size(16384);
    builderconfig = builderconfig.coins_per_utxo_byte(&utils::to_bignum(4310));
    builderconfig = builderconfig.ex_unit_prices(&ExUnitPrices::new(
        &UnitInterval::new(&utils::to_bignum(577), &utils::to_bignum(10000)),
        &UnitInterval::new(&utils::to_bignum(721), &utils::to_bignum(10000000)),
    ));
    builderconfig = builderconfig.prefer_pure_change(false);

    let builderconfig = builderconfig.build()?;
    let mut builder = TransactionBuilder::new(&builderconfig);

    // Create Plutus Datum
    let mut inner = PlutusList::new();
    inner.add(&PlutusData::new_bytes(regdat.operator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.validator_address.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.moniker.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.used_nft.name()));
    inner.add(&PlutusData::new_bytes(regdat.owner.to_bytes()));

    let datum =
        &PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&utils::to_bignum(0), &inner));
    tracing::info!("Datum: {:?}", datum);

    let mut datums = PlutusList::new();
    datums.add(datum);
    let datumhash = utils::hash_plutus_data(datum);
    tracing::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    // Create registration output containing a valid ENNFT,
    // sending the ENNFT from its source address to the smart contract and apply datum
    let mut registration_value = Value::zero();
    let mut multi_assets = MultiAsset::new();
    let mut assets = Assets::new();
    assets.insert(&valid_ennfts[0].1 .1, &valid_ennfts[0].1 .2);
    multi_assets.insert(&valid_ennfts[0].1 .0, &assets);
    registration_value.set_multiasset(&multi_assets);

    tracing::debug!("Registration Value: {:?}", registration_value);
    let mut registration_output = TransactionOutput::new(&script_address, &registration_value);
    registration_output.set_plutus_data(datum);
    let registration_output = murin::min_ada_for_utxo(&registration_output)?;
    builder.add_output(&registration_output)?;

    tracing::debug!("Policy: {:?}", valid_ennfts[0].1 .0.to_hex());
    tracing::debug!("Name: {:?}", &hex::encode(valid_ennfts[0].1 .1.name()));

    // Add required signers
    builder.add_required_signer(&pubkeyhash);

    // Metadata
    let registration_metadata = ENRegMetadata {
        operator_address: op_data.config.operator_address,
        validator_address: op_data.config.validator_address,
        moniker: op_data.config.moniker,
        ennft: ennft_fingerprint,
    };
    builder.add_json_metadatum(
        &to_bignum(9819543),
        serde_json::to_string(&registration_metadata)?,
    )?;
    tracing::debug!("Metadata: {:?}", &registration_metadata);

    builder.add_inputs_from(
        &utxos.convert_to_csl(),
        CoinSelectionStrategyCIP2::RandomImproveMultiAsset,
    )?;

    let mut diff = match input_utxo
        .output()
        .amount()
        .checked_sub(&registration_value)
    {
        Ok(amount) => amount,
        Err(_) => match registration_value.checked_sub(&input_utxo.output().amount()) {
            Ok(amount) => amount,
            Err(_) => return Err(Error::Custom("invalid inputs".to_owned())),
        },
    };

    let minada_diff = murin::calc_min_ada_for_utxo(&diff, None);
    diff.set_coin(&minada_diff);

    let mut needed = registration_value
        .checked_add(&diff)
        .map_err(MurinError::from)?
        .checked_add(&Value::new(&to_bignum(2000000)))?;
    let utxos = utxos.convert_to_csl().into_iter().collect();
    let inputs = utxo_handling::input_selection(None, &mut needed, &utxos, None, None)
        .map_err(|err| Error::Custom(err.to_string()))?;

    let mut ibuilder = TxInputsBuilder::new();

    for i in inputs.1 {
        ibuilder.add_input(&i.output().address(), &i.input(), &i.output().amount())
    }

    builder.set_inputs(&ibuilder);

    tracing::debug!("Added inputs: {:?}", &builder.get_total_input()?);
    tracing::debug!("Added outputs: {:?}", &builder.get_total_output()?);

    builder.add_change_if_needed(&first_address)?; //Address::from_hex(&tx_schema.change_address.unwrap())?

    let tx = builder.build_tx()?;
    let unsigned_tx = UnsignedTransaction {
        id: "test_id_register_earth_node".to_string(),
        tx: tx.to_hex(),
    };
    let result = BuilderResult::UnsignedTransaction(unsigned_tx);
    Ok(result)
}
