    
/*
use super::RegistrationDatum;
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
use log::debug;
use serde::{Deserialize, Serialize};
use sha2::Digest;
// Register EarthNode Transaction
//
// This module implement the registration transaction for WorldMobile's AyA Network on Cardano.
//
use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use clib::address::{BaseAddress, };
use clib::plutus::{ConstrPlutusData,  PlutusList, PlutusScripts,  Redeemer, RedeemerTag, Redeemers, Language, ExUnits};
use clib::utils::{hash_script_data};
use clib::{TransactionInputs};

use super::configuration::StakingConfig;
use super::models::StakeTxData;
use crate::cardano::{self, supporting_functions, Tokens};
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::TxData;
use crate::{min_ada_for_utxo, PerformTxb};

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtEnRegBuilder {
    /// Staking data.
    pub stxd: EnRegTxData,
    /// Configuration data.
    pub config: EnRegConfig,
}

/// This type represents a staking transaction parameters.
pub type AtEnRegParams<'param> = &'param EnRegTxData;

impl<'param> PerformTxb<AtEnRegParams<'param>> for AtEnRegBuilder {
    /// Creates new staking builder.
    fn new(params: AtEnRegParams) -> Self {
        let config = StakingConfig::load();
        Self {
            stxd: params.clone(),
            config,
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        _pvks: &[String],
        fcrun: bool,
    ) -> std::result::Result<TxBO, MurinError> {
        if fcrun {
            debug!("--------------------------------------------------------------------------------------------------------");
            debug!("-----------------------------------------Fee Calculation------------------------------------------------");
            debug!("---------------------------------------------------------------------------------------------------------\n");
        } else {
            debug!("--------------------------------------------------------------------------------------------------------");
            debug!("-----------------------------------------Build Transaction----------------------------------------------");
            debug!("--------------------------------------------------------------------------------------------------------\n");
        }
   
    //
    // Transaction Building
    //
    let mut builderconfig = TransactionBuilderConfigBuilder::new();
    builderconfig = builderconfig.fee_algo(&LinearFee::new(&to_bignum(44), &to_bignum(155381)));
    builderconfig = builderconfig.pool_deposit(&to_bignum(500000000));
    builderconfig = builderconfig.key_deposit(&to_bignum(2000000));
    builderconfig = builderconfig.max_value_size(5000);
    builderconfig = builderconfig.max_tx_size(16384);
    builderconfig = builderconfig.coins_per_utxo_byte(&to_bignum(4310));
    builderconfig = builderconfig.ex_unit_prices(&ExUnitPrices::new(
        &UnitInterval::new(&to_bignum(577), &to_bignum(10000)),
        &UnitInterval::new(&to_bignum(721), &to_bignum(10000000)),
    ));
    builderconfig = builderconfig.prefer_pure_change(false);

    let builderconfig = builderconfig.build()?;
    let mut builder = TransactionBuilder::new(&builderconfig);

    // Create Plutus Datum
    let mut inner = plutus::PlutusList::new();
    inner.add(&PlutusData::new_bytes(regdat.enOperatorAddress.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.enConsensusPubkey.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.enMerkleTreeRoot.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.enCceAddress.to_vec()));
    inner.add(&PlutusData::new_bytes(regdat.enUsedNftTn.name()));
    inner.add(&PlutusData::new_bytes(regdat.enOwner.to_bytes()));

    let datum = &plutus::PlutusData::new_constr_plutus_data(&plutus::ConstrPlutusData::new(
        &to_bignum(0),
        &inner,
    ));
    log::info!("\nDatum: {:?}", datum);

    let mut datums = plutus::PlutusList::new();
    datums.add(datum);
    let datumhash = hash_plutus_data(datum);
    log::info!("DatumHash: {:?}\n", hex::encode(datumhash.to_bytes()));

    // Create registration output containing a valid ENNFT,
    // sending the ENNFT from its source address to the smart contract and apply datum
    let mut registration_value = Value::zero();
    let mut multi_assets = MultiAsset::new();
    let mut assets = Assets::new();
    assets.insert(&valid_ennfts[0].1 .1, &valid_ennfts[0].1 .2);
    multi_assets.insert(&valid_ennfts[0].1 .0, &assets);
    registration_value.set_multiasset(&multi_assets);

    //registration_value.set_coin(&calc_min_ada_for_utxo(
    //    &registration_value,
    //    Some(datumhash),
    //)?);

    debug!("Registration Value: {:?}", registration_value);
    let mut registration_output = TransactionOutput::new(&script_address, &registration_value);
    registration_output.set_plutus_data(datum);
    let registration_output = dcslc::min_ada_for_utxo(&registration_output, &to_bignum(4310))?;
    builder.add_output(&registration_output)?;

    debug!("Policy: {:?}", valid_ennfts[0].1 .0.to_hex());
    debug!("Name: {:?}", &hex::encode(valid_ennfts[0].1 .1.name()));

    // Add required signers
    builder.add_required_signer(&pubkeyhash);

    // Metadata
    /*let registration_metadata = ENRegMetadata {
        operator_address: op_data.config.operator_address,
        validator_address: op_data.config.consensus_pub_key,
        ennft: ennft_fingerprint,
    };
    builder.add_json_metadatum(
        &to_bignum(9819543),
        serde_json::to_string(&registration_metadata)?,
    )?;
    debug!("Metadata: {:?}", &registration_metadata);
    */
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
            Err(_) => {
                return Err(TransactionBuildingError::Custom(
                    "invalid inputs".to_owned(),
                ))
            }
        },
    };

    let minada_diff = calc_min_ada_for_utxo(&diff, None)?;
    diff.set_coin(&minada_diff);

    let mut needed = registration_value
        .checked_add(&diff)?
        .checked_add(&Value::new(&to_bignum(2000000)))?;
    let inputs = dcslc::input_selection(None, &mut needed, &utxos, None, None)?;

    let mut ibuilder = TxInputsBuilder::new();

    for i in inputs.1 {
        ibuilder.add_input(&i.output().address(), &i.input(), &i.output().amount())
    }

    builder.set_inputs(&ibuilder);

    debug!("added inputs: {:?}", &builder.get_total_input()?);
    debug!("added outputs: {:?}", &builder.get_total_output()?);

    builder.add_change_if_needed(&first_address)?; //Address::from_hex(&tx_schema.change_address.unwrap())?

    let tx = builder.build_tx()?;

    Ok(BuilderResult::UnsignedTransaction(UnsignedTransaction {
        id: "test_id_register_earth_node".to_string(),
        tx: tx.to_hex(),
    }))

}}
 */