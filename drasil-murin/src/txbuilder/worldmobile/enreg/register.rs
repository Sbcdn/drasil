// Register EarthNode Transaction
//
// This module implement the registration transaction for WorldMobile's AyA Network on Cardano.
//
use super::EnRegistrationTxData;
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::TxData;
use crate::{
    calc_min_ada_for_utxo,
    cardano::{self, supporting_functions, Tokens},
    usedutxos,
    worldmobile::configuration::EnRegistrationConfig,
};
use crate::{min_ada_for_utxo, PerformTxb};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use cardano_serialization_lib::{
    address::{EnterpriseAddress, StakeCredential},
    crypto::ScriptHash,
    plutus::{self, PlutusData},
    utils::{hash_plutus_data, to_bignum, Value},
    Assets, MultiAsset, TransactionOutput,
};
use clib::plutus::{ExUnits, Language, PlutusScripts, Redeemer, RedeemerTag, Redeemers};
use clib::{
    utils::{BigInt, Int},
    TransactionOutputs,
};
use log::debug;
use supporting_functions::{balance_tx, get_input_position, get_ttl_tx};

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtEnRegBuilder {
    /// Staking data.
    pub rtxd: EnRegistrationTxData,
    /// Configuration data.
    pub config: EnRegistrationConfig,
}

/// This type represents a staking transaction parameters.
pub type AtEnRegParams<'param> = &'param EnRegistrationTxData;

impl<'param> PerformTxb<AtEnRegParams<'param>> for AtEnRegBuilder {
    /// Creates new staking builder.
    fn new(params: AtEnRegParams) -> Self {
        let config = EnRegistrationConfig::load();
        Self {
            rtxd: params.clone(),
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

        let protocol_parameters = ProtocolParameters::read_protocol_parameter(
            &std::env::var("PPPATH")
                .unwrap_or_else(|_| "protocol_parameters_preview.json".to_owned()),
        )
        .unwrap();

        // We had to fill in the first address of the sender wallet in preperation step (hugin-lib) already, we can safely unwrap here.
        let sender_address = &self.rtxd.first_addr_sender_wallet.as_ref().unwrap().clone();

        // Network determined by sending address.
        let network = sender_address.network_id()?;

        // PolicyId EnOpNft Minting Policy
        let enop_nft_minting_policy_id: ScriptHash = self.config.enop_nft_minting_policy.hash();

        // Contract Address of the Registration Validator Smart Contract
        let registration_validator_address = EnterpriseAddress::new(
            network.clone(),
            &StakeCredential::from_scripthash(
                &self.config.registration_validator_smart_contract.hash(),
            ),
        )
        .to_address();

        // Collateral UTxO had to be filled in the preperation step (hugin-lib) already, we can safely unwrap here.
        let collateral_input_txuo = gtxd.clone().get_collateral();

        // Create Plutus Datum
        let mut inner = plutus::PlutusList::new();
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_operator_address.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_consensus_pubkey.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_merkle_tree_root.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_cce_address.to_vec(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_used_nft_tn.name(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_rwd_wallet.to_bytes(),
        ));
        inner.add(&PlutusData::new_integer(&BigInt::from_str(
            &self.rtxd.registration_datum.en_commission.to_string(),
        )?));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_op_nft_cs.to_bytes().clone(),
        ));
        inner.add(&PlutusData::new_bytes(
            self.rtxd.registration_datum.en_signature.clone(),
        ));

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
        // sending the ENNFT from its source address to the enRegistration Validator smart contract and apply datum
        let mut registration_value = Value::zero();
        let mut ma_en_reg_val = MultiAsset::new();
        let mut assets = Assets::new();
        assets.insert(&self.rtxd.registration_datum.en_used_nft_tn, &to_bignum(1));
        ma_en_reg_val.insert(&self.config.ennft_policy_id, &assets);
        registration_value.set_multiasset(&ma_en_reg_val);
        let mut registration_output =
            TransactionOutput::new(&registration_validator_address, &registration_value);
        registration_output.set_plutus_data(datum);
        let registration_output = min_ada_for_utxo(&registration_output)?;

        // Create an output containing the ENOPNFT and send to senders wallet.
        let mut op_nft_value = Value::zero();
        let mut ma_op_nft_val = MultiAsset::new();
        let mut op_nft_assets = Assets::new();
        op_nft_assets.insert(&self.rtxd.registration_datum.en_used_nft_tn, &to_bignum(1));
        ma_op_nft_val.insert(&enop_nft_minting_policy_id, &op_nft_assets);
        op_nft_value.set_multiasset(&ma_op_nft_val);
        let enop_nft_output = TransactionOutput::new(&sender_address, &op_nft_value);
        let enop_nft_output = min_ada_for_utxo(&enop_nft_output)?;

        let mut txouts = TransactionOutputs::new();
        txouts.add(&enop_nft_output);
        txouts.add(&registration_output);

        let utxos = gtxd.inputs.clone();
        let ennft_utxo = self.rtxd.ennft_utxo.as_ref().unwrap().clone();
        // The ennft_utxo must exist at this point otherwise fail
        let mut diff = match ennft_utxo
            .output()
            .amount()
            .checked_sub(&registration_value)
        {
            Ok(amount) => amount,
            Err(_) => match registration_value
                .checked_sub(&self.rtxd.ennft_utxo.as_ref().unwrap().output().amount())
            {
                Ok(amount) => amount,
                Err(_) => return Err(MurinError::Custom("invalid inputs".to_owned())),
            },
        };

        let minada_diff = calc_min_ada_for_utxo(&diff, None);
        diff.set_coin(&minada_diff);

        // Lookup Input UTxOs with at least the tokens for the registration value
        let mut needed = registration_value
            // Look for enough Ada for the EnOpNft UTxO
            .checked_add(&Value::new(&op_nft_value.coin()))?
            // Enough Ada for the change UTxO
            .checked_add(&diff)?
            // Add 2 Ada reserve
            .checked_add(&Value::new(&to_bignum(2000000)))?;
        let inputs = input_selection(
            None,
            &mut needed,
            &utxos,
            collateral_input_txuo.clone(),
            None,
        )?;
        // The number of verification keys. This determine the number of signatures required.
        let saved_input_txuos = inputs.1.clone();
        let vkey_counter = cardano::get_vkey_count(&inputs.1, collateral_input_txuo.as_ref());

        let mut mint_val_zero_coin = op_nft_value.clone();
        mint_val_zero_coin.set_coin(&cutils::to_bignum(0u64));

        // Balance TX
        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut input_utxos = inputs.1.clone();

        let txouts_fin = balance_tx(
            &mut input_utxos,
            &Tokens::new(),
            &mut txouts,
            Some(mint_val_zero_coin).as_ref(), // but not the ADA!!!!
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut tbb_values,
            &sender_address,
            &sender_address,
            &mut acc,
            None,
            &fcrun,
        )?;
        let txouts_fin = combine_wallet_outputs(&txouts_fin);

        ////////////////////////////////////////////////////////////////////////////////////////////
        //
        // MINT ASSETS
        //
        ////////////////////////////////////////////////////////////////////////////////////////////
        let mut mintasset = clib::MintAssets::new();
        mintasset.insert(
            &self.rtxd.registration_datum.en_used_nft_tn,
            Int::new_i32(1),
        );
        let mint = clib::Mint::new_from_entry(&enop_nft_minting_policy_id, &mintasset);
        let slot = cutils::to_bignum(
            gtxd.clone().get_current_slot() + get_ttl_tx(&gtxd.clone().get_network()),
        );

        // CostModel
        let cost_models = protocol_parameters.get_CostMdls().unwrap();
        let costmodel = cost_models.get(&Language::new_plutus_v2()).unwrap();
        let mut pcm = plutus::CostModel::new();
        let mut c = 0;
        for i in 0..costmodel.len() {
            if let Ok(n) = costmodel.get(i) {
                if n != cutils::Int::from_str("0")? {
                    pcm.set(c, &n)?;
                    c += 1;
                }
            }
        }
        let mut cstmodls = plutus::Costmdls::new();
        cstmodls.insert(&plutus::Language::new_plutus_v2(), &pcm);

        // Redeemer
        let redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(0u64), &plutus::PlutusList::new()),
        );

        let exunits = ExUnits::new(
            &to_bignum(protocol_parameters.execution_unit_prices.priceMemory as u64),
            &to_bignum(protocol_parameters.execution_unit_prices.priceSteps as u64),
        );
        let redeemer_index = get_input_position(
            inputs.0.clone(),
            self.rtxd.ennft_utxo.as_ref().unwrap().clone(),
        );

        let redeemer = Redeemer::new(
            &RedeemerTag::new_mint(),
            &to_bignum(redeemer_index.0 as u64),
            &redeemer_data,
            &exunits,
        );

        let mut redeemers = Redeemers::new();
        redeemers.add(&redeemer);
        log::debug!("\nCostModels:\n{:?}\n\n", cstmodls);

        let scriptdatahash = cutils::hash_script_data(&redeemers, &cstmodls, Some(datums.clone()));
        log::debug!(
            "ScriptDataHash: {:?}\n",
            hex::encode(scriptdatahash.to_bytes())
        );

        let mut txbody = clib::TransactionBody::new_tx_body(&inputs.0, &txouts_fin, fee);
        txbody.set_ttl(&slot);
        txbody.set_mint(&mint);
        txbody.set_script_data_hash(&scriptdatahash);

        // Create the transaction witnesses.
        let mut txwitness = clib::TransactionWitnessSet::new();
        txwitness.set_redeemers(&redeemers);
        txwitness.set_plutus_data(&datums);
        let mut plutus_scripts = PlutusScripts::new();
        plutus_scripts.add(&self.config.enop_nft_minting_policy);
        txwitness.set_plutus_scripts(&plutus_scripts);

        Ok((txbody, txwitness, None, saved_input_txuos, vkey_counter))
    }
}
