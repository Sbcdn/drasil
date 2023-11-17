//! Un-staking transaction
//!
//! This module implements un-staking transaction for WorldMobile token.

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use clib::plutus::{self, Redeemer, RedeemerTag, Redeemers, PlutusData};
use clib::plutus::{ExUnits, Language, PlutusScripts};
use clib::utils::{hash_script_data, to_bignum};
use clib::{AssetName, Assets, MultiAsset};
use clib::{TransactionInputs, TransactionOutput, TransactionOutputs};

use super::UnStakeTxData;
use crate::cardano::{self, supporting_functions, Tokens};
use crate::error::MurinError;
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::pparams::ProtocolParameters;
use crate::txbuilder::{input_selection, TxBO};
use crate::worldmobile::configuration::StakingConfig;
use crate::{min_ada_for_utxo, PerformTxb, TxData, get_input_position};

/// This type is a staking transaction builder for WMT.
#[derive(Debug, Clone)]
pub struct AtUnStakingBuilder {
    /// Un-staking data.
    pub unstake_data: UnStakeTxData,
    /// Configuration data.
    pub config: StakingConfig,
}

impl PerformTxb<&UnStakeTxData> for AtUnStakingBuilder {
    /// Creates new un-staking builder.
    fn new(unstake_data: &UnStakeTxData) -> Self {
        let config = StakingConfig::load();
        Self {
            unstake_data: unstake_data.clone(),
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

        if self.config.smart_contracts.len() != 2usize {
            return Err(MurinError::new("Minting policy or validator are missing"));
        }

        let wallet_addr = self
            .unstake_data
            .wallet_addr
            .as_ref()
            .ok_or_else(|| MurinError::new("unable to get wallet address"))?;

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        // Add Inputs and Outputs
        ///////////////////////////////////////////////////////////////////////////////////////////////////////

        
        // We need to specify our Transaction Outputs (TxO), in this case
        // our TxO is only one, the one we send from the staking validator smart contract address to the users wallet address.
        // We know that this must at least be the WMT and Ada on the staking UTxO (staking UtxO is verified by the validator smart contract).
        // We also know that there is a execution proof NFT on the the staking UTxO, which must be burned in this transaction
        // and will because of that not be in the TxO anymore.
        // We construct a 'cutils::Value' only contaning the execution proof NFT and substract it from the 'cutils::Value' of the staking UTxO.
        // Restore PolicyId from Plutus Minting Policy
        let minting_policy_ex_proof = self
            .config
            .smart_contracts
            .get("minting")
            .ok_or_else(|| MurinError::new("failed to get minting smart contract"))?;
        let execution_proof_policy_id = minting_policy_ex_proof.hash();

        // We take the TokenName (or AssetName in CSL terms) from the input UTxO (which is our staking UTxO)
        let execution_token_name_hex = self
            .unstake_data
            .transaction_input
            .input()
            .transaction_id()
            .to_hex();
        let execution_token_name = AssetName::from_hex(&execution_token_name_hex)?;

        // We construct new Tokens on our UTxO and we use a multiasset to wrap them
        let mut multiassets = MultiAsset::new();
        let mut assets = Assets::new();
        // We Construct the inner Map from MultiAsset the "assets"
        assets.insert(&execution_token_name, &cutils::to_bignum(0));
        // We construct the MultiAsset (Map PolicyId (Map AssesName Amount))
        multiassets.insert(&execution_proof_policy_id, &assets);

        // This value contains only the exwecution proof NFT which will be burned in this transaction
        // So we do not want to have it in the outputs
        let mut burn_value = clib::utils::Value::zero();
        burn_value.set_multiasset(&multiassets);

        // We can construct the TxO by substracting the burn value from the staking UTxO value
        // as we know that at least the amount of WMT on the input UTxO must be returned to the wallet, as well as the minUTxO Ada but not the ExProof NFT.
        let staking_input_value = self.unstake_data.transaction_input.output().amount();

        let mut wallet_value = staking_input_value.checked_sub(&burn_value)?;

        let wallet_output = TransactionOutput::new(self.unstake_data.wallet_addr.as_ref().unwrap(), &wallet_value);
        let min_utxo_val = min_ada_for_utxo(&wallet_output)?.amount().coin();
        wallet_value.set_coin(&min_utxo_val);
        let wallet_output = TransactionOutput::new(self.unstake_data.wallet_addr.as_ref().unwrap(), &wallet_value);

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
        let mut needed_value = burn_value.clone();
        needed_value.set_coin(&needed_value.coin().checked_add(&fee.clone())?);
        let security = cutils::to_bignum(
            cutils::from_bignum(&needed_value.coin()) / 100 * 10 + (2 * cardano::MIN_ADA),
        ); // 10% Security for min utxo etc.
        needed_value.set_coin(&needed_value.coin().checked_add(&security)?);

        debug!("Needed Value: {:?}", needed_value);
        debug!("\n\nTxIns Before selection:\n {:?}\n\n", input_txuos);

        let (txins, mut input_txuos) = input_selection(
            None,
            &mut needed_value,
            &input_txuos,
            gtxd.clone().get_collateral(),
            None, //Some(native_script_address).as_ref(),
        )?;

        let saved_input_txuos = input_txuos.clone();
        info!("Saved Inputs: {:?}", saved_input_txuos);

        let mut txouts = TransactionOutputs::new();
        txouts.add(&wallet_output);

        // The number of verification keys. This determine the number of signatures required.
        let vkey_counter = cardano::get_vkey_count(&input_txuos, collateral_input_txuo.as_ref());

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
            None,
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
        // MINT ASSETS
        ////////////////////////////////////////////////////////////////////////////////////////////

        // Create the burn information for this transacion (Look into TxInfo::TxMint for more information)
        let mut mintasset = clib::MintAssets::new();
        mintasset.insert(&execution_token_name, cutils::Int::new_i32(-1));
        let mint = clib::Mint::new_from_entry(&execution_proof_policy_id, &mintasset);

        // This is the TTL for the transaction on the current network.
        let ttl = cardano::get_ttl_tx(&gtxd.clone().get_network());
        // Current slot of the network.
        let slot = cutils::to_bignum(gtxd.clone().get_current_slot() + ttl);

        // Create the transaction body.
        let mut txbody = clib::TransactionBody::new_tx_body(&txins, &txouts_fin, fee);
        let mut ref_inputs = TransactionInputs::new();
        let ref_input = self
            .unstake_data
            .registration_reference
            .as_ref()
            .map(|r| r.input())
            .ok_or_else(|| MurinError::new("unable to get registration reference"))?;
        ref_inputs.add(&ref_input);
        txbody.set_reference_inputs(&ref_inputs);
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

        // Lets create the Minting Policy Redeemer to burn the Execution Proof NFT
        let mut inner = plutus::PlutusList::new();
        inner.add(&PlutusData::new_bytes(execution_token_name.name()));

        // Create the redeemer data for the burning of the execution proof
        let redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(1), &inner),
        );
        // We need to find the right position of our staking UTxO in the transaction inputs and 
        // pass it into the Redeemer, so the smart contract knows what UTxO is must verify. 
        let index = get_input_position(txins, self.unstake_data.transaction_input.clone());
        // Construct the final redeemer for burning the execution proof
        let redeemer1 = Redeemer::new(
            &RedeemerTag::new_mint(),
            &to_bignum(index.0 as u64),
            &redeemer_data,
            &exunits,
        );


        // Lets create the validator Redeemer to spent the UTxO
        // Create the redeemer data for spending the staking UTxO
        let redeemer_data = plutus::PlutusData::new_constr_plutus_data(
            &plutus::ConstrPlutusData::new(&to_bignum(0), &plutus::PlutusList::new()),
        );
        // Construct the final redeemer for spending the staking UTxO
        let redeemer2 = Redeemer::new(
            &RedeemerTag::new_spend(),
            &to_bignum(index.0 as u64),
            &redeemer_data,
            &exunits,
        );


        let mut redeemers = Redeemers::new();
        redeemers.add(&redeemer1);
        redeemers.add(&redeemer2);
        log::debug!("\nCostModels:\n{:?}\n\n", cstmodls);

        let scriptdatahash = hash_script_data(&redeemers, &cstmodls, None);
        log::debug!(
            "ScriptDataHash: {:?}\n",
            hex::encode(scriptdatahash.to_bytes())
        );

        let validator_contract = self
        .config
        .smart_contracts
        .get("validator")
        .ok_or_else(|| MurinError::new("validator does not exist"))?;

        // Create the transaction witnesses.
        let mut txwitness = clib::TransactionWitnessSet::new();
        txwitness.set_redeemers(&redeemers);
        let mut plutus_scripts = PlutusScripts::new();
        plutus_scripts.add(minting_policy_ex_proof);
        plutus_scripts.add(validator_contract);
        txwitness.set_plutus_scripts(&plutus_scripts);

        Ok((txbody, txwitness, None, saved_input_txuos, vkey_counter))
    }
}
