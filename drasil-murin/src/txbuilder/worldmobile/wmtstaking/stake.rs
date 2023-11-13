//! Stake transaction
//!
//! This module implement the staking transaction for WorldMobile token.
//!

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use clib::address::{BaseAddress, EnterpriseAddress, StakeCredential};
use clib::plutus::{
    self, ConstrPlutusData, ExUnits, Language, PlutusData, PlutusList, PlutusScripts, Redeemer,
    RedeemerTag, Redeemers,
};
use clib::utils::{hash_script_data, to_bignum};
use clib::{
    AssetName, Assets, MultiAsset, TransactionInputs, TransactionOutput, TransactionOutputs,
};

use crate::cardano::{self, supporting_functions, Tokens};
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::worldmobile::configuration::StakingConfig;
use crate::TxData;
use crate::{min_ada_for_utxo, PerformTxb};

use super::StakeTxData;

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtStakingBuilder {
    /// Staking data.
    pub stxd: StakeTxData,
    /// Configuration data.
    pub config: StakingConfig,
}

/// This type represents a staking transaction parameters.
pub type AtStakingParams<'param> = &'param StakeTxData;

impl<'param> PerformTxb<AtStakingParams<'param>> for AtStakingBuilder {
    /// Creates new staking builder.
    fn new(params: AtStakingParams) -> Self {
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

        if self.config.smart_contracts.len() != 1usize {
            return Err(MurinError::new(
                "Minting multiple NFTs from different scripts isn't yet supported",
            ));
        }

        let validator_contract = self
            .config
            .smart_contracts
            .get("validator")
            .ok_or_else(|| MurinError::new("validator does not exist"))?;
        let script_hash = validator_contract.hash();
        let credential = StakeCredential::from_scripthash(&script_hash);

        let wallet_addr = self
            .stxd
            .wallet_addr
            .as_ref()
            .ok_or_else(|| MurinError::new("unable to get wallet address"))?;
        let network_id = wallet_addr.network_id()?;
        let contract_address = EnterpriseAddress::new(network_id, &credential).to_address();

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        // This variable is named "ma" in other places.
        let mut multiassets = MultiAsset::new();
        let mut assets = Assets::new();

        // Key is the asset name, should come from configuration data.
        assets.insert(
            &self.config.wmt_assetname,
            &cutils::to_bignum(self.stxd.staking_amount),
        );
        // We create a policy from the policy in configuration data.
        multiassets.insert(&self.config.wmt_policy_id, &assets);

        // Create new asset for execution proof.
        // Dummy token name is needed to resolve the correct token name
        // because we know the size of the token name which is a transaction hash(32bits) of
        // an input UTXO. But Input UTxOs are not select yet so we use a dummy to first select tokens and still have a correct minUtxO calculation.
        let dummy_token_name =
            hex::decode("b9df48c3f4614337d7c67fc4ecd81e404cc75b89ef348a92e6d34f02a70b242e")?;

        // Restore PolicyId from Plutus Minting Policy
        let minting_policy_ex_proof = self
            .config
            .smart_contracts
            .get("minting")
            .ok_or_else(|| MurinError::new("failed to get minting smart contract"))?;
        let execution_proof_policy_id = minting_policy_ex_proof.hash();

        let mut assets = Assets::new();
        let key = AssetName::new(dummy_token_name.clone())?;
        assets.insert(&key, &cutils::to_bignum(1));
        multiassets.insert(&execution_proof_policy_id, &assets);

        // This value goes to the validator address.
        let mut validator_value = clib::utils::Value::zero();
        validator_value.set_multiasset(&multiassets);

        // Create Plutus Datum
        let mut inner = PlutusList::new();
        inner.add(&PlutusData::new_bytes(hex::decode(&self.stxd.ennft)?));
        // With a staking address we can determine which addresses belong to the
        // same wallet.
        let base_addr = BaseAddress::from_address(wallet_addr)
            .ok_or_else(|| MurinError::new("fail to create a base address from valid address"))?;
        let payment_credential = base_addr
            .payment_cred()
            .to_keyhash()
            .ok_or_else(|| MurinError::new("failed to get payment key hash"))?;
        inner.add(&PlutusData::new_bytes(payment_credential.to_bytes()));
        inner.add(&PlutusData::new_bytes(execution_proof_policy_id.to_bytes()));
        inner.add(&PlutusData::new_bytes(dummy_token_name.clone()));
        let datum =
            &PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&to_bignum(0), &inner));

        let mut validator_output = TransactionOutput::new(&contract_address, &validator_value);
        validator_output.set_plutus_data(datum);
        let min_utxo_val = min_ada_for_utxo(&validator_output)?.amount().coin();
        validator_value.set_coin(&min_utxo_val);

        // Get the input UTxOS values
        let mut input_txuos = gtxd.clone().get_inputs();
        info!("\n Before USED UTXOS");
        // Check if some utxos in inputs are in a pending transaction and remove them
        if let Some(used_utxos) = crate::utxomngr::usedutxos::check_any_utxo_used(&input_txuos)? {
            info!("\n\n");
            info!("USED UTXOS: {:?}", used_utxos);
            info!("\n\n");
            input_txuos.remove_used_utxos(used_utxos);
        }

        let collateral_input_txuo = gtxd.clone().get_collateral();
        debug!("\nCollateral Input: {:?}", collateral_input_txuo);

        // The required value for the transaction.
        let mut needed_value = validator_value.clone();
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone())?);
        let security = cutils::to_bignum(
            cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * cardano::MIN_ADA),
        ); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security)?);

        debug!("Needed Value: {:?}", needed_value);
        debug!("\n\nTxIns Before selection:\n {:?}\n\n", input_txuos);

        // Remove execution proof from needed value.
        let mut multiasset = MultiAsset::new();
        let mut assets = Assets::new();
        let asset_name = AssetName::new(dummy_token_name)?;
        assets.insert(&asset_name, &to_bignum(1));
        multiasset.insert(&execution_proof_policy_id, &assets);
        let mut execution_value = cutils::Value::zero();
        execution_value.set_multiasset(&multiasset);
        let mut needed_value = needed_value.checked_sub(&execution_value)?;

        let (txins, mut input_txuos) = input_selection(
            None,
            &mut needed_value,
            &input_txuos,
            gtxd.clone().get_collateral(),
            None, //Some(native_script_address).as_ref(),
        )?;

        let saved_input_txuos = input_txuos.clone();
        info!("Saved Inputs: {:?}", saved_input_txuos);

        // Replace dummy token name with real execution token name.
        // txins only comes from the wallet and we have to have atleast one.
        // we use the input at index 0 because it must exist.
        let tx_input = txins.get(0);
        let exec_token_name = tx_input.transaction_id().to_bytes();
        let _exec_token_index = tx_input.index();

        // Reconstruct the datum with the execution token name and minting value.
        // Create Plutus Datum
        let mut inner = PlutusList::new();
        inner.add(&PlutusData::new_bytes(hex::decode(&self.stxd.ennft)?));

        let payment_credential = base_addr
            .payment_cred()
            .to_keyhash()
            .ok_or_else(|| MurinError::new("failed to get payment key hash"))?;
        inner.add(&PlutusData::new_bytes(payment_credential.to_bytes()));
        inner.add(&PlutusData::new_bytes(execution_proof_policy_id.to_bytes()));
        inner.add(&PlutusData::new_bytes(exec_token_name.clone()));
        let datum =
            &PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&to_bignum(0), &inner));

        // The validator asset has WMT, proof and Ada.
        let validator_multiasset = validator_value
            .multiasset()
            .ok_or_else(|| MurinError::new("failed to get multiasset"))?;
        // Remove the dummy execution token from the multiasset.
        let mut validator_multiasset = validator_multiasset.sub(&multiasset);

        // Create new asset name with the execution token name.
        let asset_name = AssetName::new(exec_token_name)?;

        // The value is one because it's an NFT
        validator_multiasset.set_asset(&execution_proof_policy_id, &asset_name, to_bignum(1));
        validator_value.set_multiasset(&validator_multiasset);

        let mut validator_output = TransactionOutput::new(&contract_address, &validator_value);
        validator_output.set_plutus_data(datum);

        let mut txouts = TransactionOutputs::new();
        txouts.add(&validator_output);

        // The number of verification keys. This determine the number of signatures required.
        let vkey_counter = cardano::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref());

        // This is the value of the execution proof.
        let mut mint_multiasset = MultiAsset::new();
        mint_multiasset.set_asset(&execution_proof_policy_id, &asset_name, to_bignum(1));

        let mut mint_val_zero_coin = cutils::Value::zero();
        mint_val_zero_coin.set_multiasset(&mint_multiasset);

        // Balance TX
        let mut fee_paid = false;
        let mut first_run = true;
        let mut txos_paid = false;
        let mut tbb_values = cutils::Value::new(&cutils::to_bignum(0u64));
        let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));

        // This is the final output to use in the transaction.
        let txouts_fin = supporting_functions::balance_tx(
            &mut input_txuos,
            &Tokens::new(),
            &mut txouts,
            Some(mint_val_zero_coin).as_ref(), // but not the ADA!!!!
            fee,
            &mut fee_paid,
            &mut first_run,
            &mut txos_paid,
            &mut tbb_values,
            wallet_addr,
            wallet_addr,
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

        // This is the TxMintInfo
        let mut mintasset = clib::MintAssets::new();
        mintasset.insert(&asset_name, cutils::Int::new_i32(1));
        let mint = clib::Mint::new_from_entry(&execution_proof_policy_id, &mintasset);

        // This is the TTL for the transaction on the current network.
        let ttl = cardano::get_ttl_tx(&gtxd.clone().get_network());
        // Current slot of the network.
        let slot = cutils::to_bignum(gtxd.clone().get_current_slot() + ttl);

        // Create the transaction body.
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        let mut inputs = TransactionInputs::new();
        let input = self
            .stxd
            .registration_reference
            .as_ref()
            .map(|r| r.input())
            .ok_or_else(|| MurinError::new("unable to get registration reference"))?;
        inputs.add(&input);
        txbody.set_reference_inputs(&inputs);
        txbody.set_ttl(&slot);
        txbody.set_mint(&mint);

        // Read the protocol parameters.
        let protocol_parameters =
            ProtocolParameters::read_protocol_parameter(&self.config.protocol_param_path)?;

        // Get the cost model.
        let cost_models = protocol_parameters.get_CostMdls()?;
        let costmodel = cost_models
            .get(&Language::new_plutus_v2())
            .ok_or_else(|| MurinError::new("failed to get cost models"))?;

        // Create new plutus cost model
        let mut plutus_cost_model = plutus::CostModel::new();
        let mut c = 0;
        for i in 0..costmodel.len() {
            if let Ok(n) = costmodel.get(i) {
                if n != cutils::Int::from_str("0")? {
                    plutus_cost_model.set(c, &n)?;
                    c += 1;
                }
            }
        }
        let mut cstmodls = plutus::Costmdls::new();
        cstmodls.insert(&plutus::Language::new_plutus_v2(), &plutus_cost_model);

        let exunits = ExUnits::new(
            &to_bignum(protocol_parameters.execution_unit_prices.priceMemory as u64),
            &to_bignum(protocol_parameters.execution_unit_prices.priceSteps as u64),
        );

        // Create the redeemer data for the transaction.
        let redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(1), &plutus::PlutusList::new()),
        );
        // Add the redeemer data to the redeemer.
        let redeemer = Redeemer::new(
            &RedeemerTag::new_mint(),
            &to_bignum(0),
            &redeemer_data,
            &exunits,
        );
        let mut redeemers = Redeemers::new();
        redeemers.add(&redeemer);
        log::debug!("\nCostModels:\n{:?}\n\n", cstmodls);

        let scriptdatahash = hash_script_data(&redeemers, &cstmodls, None);
        log::debug!(
            "ScriptDataHash: {:?}\n",
            hex::encode(scriptdatahash.to_bytes())
        );

        // Create the transaction witnesses.
        let mut txwitness = clib::TransactionWitnessSet::new();
        txwitness.set_redeemers(&redeemers);
        let mut plutus_scripts = PlutusScripts::new();
        plutus_scripts.add(minting_policy_ex_proof);
        txwitness.set_plutus_scripts(&plutus_scripts);

        Ok((txbody, txwitness, None, saved_input_txuos, vkey_counter))
    }
}
