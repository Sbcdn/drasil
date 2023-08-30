/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::{models, supporting_functions};
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{address as caddr, crypto as ccrypto, utils as cutils};
use clib::address::Address;
use clib::utils::{to_bignum, BigNum};
use clib::{NetworkIdKind, TransactionOutput};
use serde::{Deserialize, Serialize};
use std::ops::{Div, Rem, Sub};

use crate::error::MurinError;
pub mod delegation;
pub mod deregistration;
pub mod finalize;
pub mod marketplace;
pub mod minter;
pub mod modules;
pub mod rwdist;
pub mod stdtx;

pub use delegation::DelegTxData;
pub use marketplace::*;
pub use rwdist::*;

use crate::{TransactionUnspentOutput, TransactionUnspentOutputs};

type TxBO = (
    clib::TransactionBody,
    clib::TransactionWitnessSet,
    Option<clib::metadata::AuxiliaryData>,
    TransactionUnspentOutputs,
    usize,
);

pub trait PerformTxb<T> {
    fn new(t: T) -> Self;

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        pvks: &[String],
        fcrun: bool,
    ) -> std::result::Result<TxBO, MurinError>;
}

#[derive(Debug, Clone)]
pub struct TxBuilder {
    pub gtxd: TxData,
    pub pvks: Vec<String>,
}

impl TxBuilder {
    pub fn new(gtxd: &TxData, pvks: &Vec<String>) -> Self {
        TxBuilder {
            gtxd: gtxd.clone(),
            pvks: pvks.to_owned(),
        }
    }

    pub async fn build<P, A: PerformTxb<P>>(
        &self,
        app_type: &A,
    ) -> Result<crate::BuildOutput, MurinError> {
        // Temp until Protocol Parameters fixed
        let mem = cutils::to_bignum(7000000u64);
        let steps = cutils::to_bignum(2500000000u64);
        let ex_unit_price: models::ExUnitPrice = crate::ExUnitPrice {
            priceSteps: 7.21e-5,
            priceMemory: 5.77e-2,
        };
        let a = cutils::to_bignum(44u64);
        let b = cutils::to_bignum(155381u64);
        //Create first Tx
        let mut tx_ =
            app_type.perform_txb(&cutils::to_bignum(2000000), &self.gtxd, &self.pvks, true)?;
        let dummy_vkeywitnesses = supporting_functions::make_dummy_vkeywitnesses(tx_.4);
        tx_.1.set_vkeys(&dummy_vkeywitnesses);
        // Build and encode dummy transaction
        let transaction_ = clib::Transaction::new(&tx_.0, &tx_.1, tx_.2);
        let calculated_fee = supporting_functions::calc_txfee(
            &transaction_,
            &a,
            &b,
            ex_unit_price.clone(),
            &steps,
            &mem,
            true,
        );
        // (txbody, txwitness, aux_data, used_utxos, vkey_counter_2)
        let tx = app_type.perform_txb(&calculated_fee, &self.gtxd, &self.pvks, false)?;

        let transaction2 = clib::Transaction::new(&tx.0, &tx_.1, tx.2.clone());

        if tx.4 != tx_.4 || transaction2.to_bytes().len() != transaction_.to_bytes().len() {
            let dummy_vkeywitnesses = supporting_functions::make_dummy_vkeywitnesses(tx.4);
            tx_.1.set_vkeys(&dummy_vkeywitnesses);

            let calculated_fee = supporting_functions::calc_txfee(
                &transaction2,
                &a,
                &b,
                ex_unit_price,
                &steps,
                &mem,
                true,
            );
            let tx = app_type.perform_txb(&calculated_fee, &self.gtxd, &self.pvks, false)?;
            info!("Fee: {:?}", calculated_fee);
            Ok(supporting_functions::tx_output_data(
                tx.0,
                tx.1,
                tx.2,
                tx.3.to_hex()?,
                0u64,
                false,
            )?)
        } else {
            info!("Fee: {:?}", calculated_fee);
            Ok(supporting_functions::tx_output_data(
                tx.0,
                tx.1,
                tx.2,
                tx.3.to_hex()?,
                0u64,
                false,
            )?)
        }
    }
}

pub type TokenAsset = (clib::PolicyID, clib::AssetName, cutils::BigNum);
pub type MintTokenAsset = (Option<clib::PolicyID>, clib::AssetName, cutils::BigNum);

#[derive(Debug, Clone)]
pub struct TxData {
    user_id: Option<i64>,
    contract_id: Option<Vec<i64>>,
    senders_addresses: Vec<caddr::Address>,
    senders_stake_addr: caddr::Address,
    outputs: Option<TransactionUnspentOutputs>,
    inputs: TransactionUnspentOutputs,
    excludes: Option<TransactionUnspentOutputs>,
    collateral: Option<TransactionUnspentOutput>,
    network: clib::NetworkIdKind,
    current_slot: u64,
}

const LV_PLUTUSV1           : &str = "a141005901d59f1a000302590001011a00060bc719026d00011a000249f01903e800011a000249f018201a0025cea81971f70419744d186419744d186419744d186419744d186419744d186419744d18641864186419744d18641a000249f018201a000249f018201a000249f018201a000249f01903e800011a000249f018201a000249f01903e800081a000242201a00067e2318760001011a000249f01903e800081a000249f01a0001b79818f7011a000249f0192710011a0002155e19052e011903e81a000249f01903e8011a000249f018201a000249f018201a000249f0182001011a000249f0011a000249f0041a000194af18f8011a000194af18f8011a0002377c190556011a0002bdea1901f1011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000242201a00067e23187600010119f04c192bd200011a000249f018201a000242201a00067e2318760001011a000242201a00067e2318760001011a0025cea81971f704001a000141bb041a000249f019138800011a000249f018201a000302590001011a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a000249f018201a00330da70101ff";
//const LV_PLUTUSV2: &str = "";

impl TxData {
    pub fn new(
        contract_id: Option<Vec<i64>>,
        saddresses: Vec<caddr::Address>,
        sstake: Option<caddr::Address>,
        inputs: TransactionUnspentOutputs,
        network: clib::NetworkIdKind,
        current_slot: u64,
    ) -> Result<TxData, MurinError> {
        let sa = match sstake {
            Some(stake) => stake,
            None => crate::cip30::reward_address_from_address(&saddresses[0])?,
        };
        Ok(TxData {
            user_id: None,
            contract_id,
            senders_addresses: saddresses,
            senders_stake_addr: sa,
            inputs,
            network,
            outputs: None,
            excludes: None,
            collateral: None,
            current_slot,
        })
    }

    pub fn set_user_id(&mut self, user_id: i64) {
        self.user_id = Some(user_id);
    }

    pub fn set_contract_id(&mut self, contract_ids: Vec<i64>) {
        self.contract_id = Some(contract_ids);
    }

    pub fn set_senders_addresses(&mut self, addresses: Vec<caddr::Address>) {
        self.senders_addresses = addresses;
    }

    pub fn set_stake_address(&mut self, address: caddr::Address) {
        self.senders_stake_addr = address;
    }

    pub fn set_outputs(&mut self, outputs: TransactionUnspentOutputs) {
        self.outputs = Some(outputs);
    }

    pub fn set_inputs(&mut self, inputs: TransactionUnspentOutputs) {
        self.inputs = inputs;
    }

    pub fn set_excludes(&mut self, excludes: TransactionUnspentOutputs) {
        self.excludes = Some(excludes);
    }

    pub fn set_collateral(&mut self, collateral: TransactionUnspentOutput) {
        self.collateral = Some(collateral);
    }

    pub fn set_current_slot(&mut self, current_slot: u64) {
        self.current_slot = current_slot;
    }

    pub fn get_user_id(&self) -> Option<i64> {
        self.user_id
    }

    pub fn get_contract_id(&self) -> Option<Vec<i64>> {
        self.contract_id.clone()
    }

    pub fn get_senders_addresses(&self) -> Vec<caddr::Address> {
        self.senders_addresses.clone()
    }

    pub fn get_senders_address(&self, i: Option<usize>) -> Option<caddr::Address> {
        match self.senders_addresses.len() {
            0 => None,
            _ => match i {
                Some(n) => {
                    if n < self.senders_addresses.len() {
                        Some(self.senders_addresses[n].clone())
                    } else {
                        None
                    }
                }
                None => Some(self.senders_addresses[0].clone()),
            },
        }
    }

    pub fn get_stake_address(&self) -> caddr::Address {
        self.senders_stake_addr.clone()
    }

    pub fn get_outputs(&self) -> Option<TransactionUnspentOutputs> {
        self.outputs.clone()
    }

    pub fn get_inputs(&self) -> TransactionUnspentOutputs {
        self.inputs.clone()
    }

    pub fn get_excludes(&self) -> Option<TransactionUnspentOutputs> {
        self.excludes.clone()
    }

    pub fn get_collateral(&self) -> Option<TransactionUnspentOutput> {
        self.collateral.clone()
    }

    pub fn get_network(&self) -> NetworkIdKind {
        self.network
    }

    pub fn get_current_slot(&self) -> u64 {
        self.current_slot
    }
}

impl ToString for TxData {
    fn to_string(&self) -> String {
        let mut s_senders_addresses = String::new();
        for a in self.get_senders_addresses() {
            s_senders_addresses.push_str(&(hex::encode(a.to_bytes()) + "?"));
            trace!("Addresses ToString TxData: {:?}", s_senders_addresses);
        }
        s_senders_addresses.pop();

        //prepare stake address
        let s_senders_stake_addr = match self.get_stake_address().to_bech32(None) {
            Ok(addr) => addr,
            _ => "".to_string(),
        };

        //prepare outputs
        let mut s_outputs = String::new();
        match self.get_outputs() {
            Some(o) => {
                if let Ok(ok) = o.to_hex() {
                    s_outputs = ok
                }
            }
            _ => s_outputs = "NoData".to_string(),
        }

        // prepare inputs
        let mut s_inputs = String::new();
        if let Ok(i) = self.get_inputs().to_hex() {
            s_inputs = i
        }

        //prepare excludes
        let mut s_excludes = String::new();
        match self.get_excludes() {
            Some(ex) => {
                if let Ok(ok) = ex.to_hex() {
                    s_excludes = ok
                }
            }
            _ => s_excludes = "NoData".to_string(),
        }

        //prepare collateral
        let s_collateral = match self.get_collateral() {
            Some(ex) => hex::encode(ex.to_bytes()),
            _ => "NoData".to_string(),
        };

        let s_network = match self.get_network() {
            clib::NetworkIdKind::Mainnet => "mainnet".to_string(),
            clib::NetworkIdKind::Testnet => "testnet".to_string(),
        };

        let s_user_id = match self.get_user_id() {
            Some(uid) => uid.to_string(),
            None => "NoData".to_string(),
        };

        let s_contract_id = match self.get_contract_id() {
            Some(cid) => {
                let mut s = String::new();
                for i in cid {
                    s.push_str(&i.to_string());
                    s.push(',');
                }
                s.pop();
                s
            }
            None => "NoData".to_string(),
        };

        let mut ret = "".to_string();
        ret.push_str(&s_senders_addresses);
        ret.push('|');
        ret.push_str(&s_senders_stake_addr);
        ret.push('|');
        ret.push_str(&s_outputs);
        ret.push('|');
        ret.push_str(&s_inputs);
        ret.push('|');
        ret.push_str(&s_excludes);
        ret.push('|');
        ret.push_str(&s_collateral);
        ret.push('|');
        ret.push_str(&s_network);
        ret.push('|');
        ret.push_str(&self.get_current_slot().to_string());
        ret.push('|');
        ret.push_str(&s_user_id);
        ret.push('|');
        ret.push_str(&s_contract_id);

        ret
    }
}

impl core::str::FromStr for TxData {
    type Err = MurinError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let slice: Vec<&str> = src.split('|').collect();
        if slice.len() == 10 {
            // restore senders addresses
            let mut senders_addresses = Vec::<caddr::Address>::new();
            let slice_addresses: Vec<&str> = slice[0].split('?').collect();
            debug!("Slice Addresses TxData: {:?}", slice_addresses);
            for addr in slice_addresses {
                senders_addresses.push(caddr::Address::from_bytes(hex::decode(addr)?)?);
            }

            // restore stake address
            let strake_address = caddr::Address::from_bech32(slice[1])?;

            // restore outputs
            let outputs = match slice[2] {
                "NoData" => None,
                _ => Some(TransactionUnspentOutputs::from_hex(slice[2])?),
            };

            // restore inputs
            let inputs = TransactionUnspentOutputs::from_hex(slice[3])?;

            // restore excludes
            let excludes = match slice[4] {
                "NoData" => None,
                _ => Some(TransactionUnspentOutputs::from_hex(slice[4])?),
            };

            // restore collateral
            let collateral = match slice[5] {
                "NoData" => None,
                _ => Some(TransactionUnspentOutput::from_bytes(hex::decode(
                    slice[5],
                )?)?),
            };

            //restore network
            let network = match slice[6] {
                "mainnet" => clib::NetworkIdKind::Mainnet,
                "testnet" => clib::NetworkIdKind::Testnet,
                _ => {
                    return Err(MurinError::new(
                        "ERROR network could not be restored from string",
                    ))
                }
            };

            // restore current slot
            let curr_slot = slice[7].parse::<u64>()?;

            let user_id = match slice[8] {
                "NoData" => None,
                _ => Some(slice[8].parse::<i64>()?),
            };

            let contract_id = match slice[9] {
                "NoData" => None,
                _ => {
                    let scids: Vec<&str> = slice[9].split(',').collect();
                    let mut cids = Vec::<i64>::new();
                    scids.iter().for_each(|n| {
                        cids.push(
                            n.parse::<i64>()
                                .expect("could not convert string to contract-id"),
                        )
                    });
                    Some(cids)
                }
            };

            Ok(TxData {
                user_id,
                contract_id,
                senders_addresses,
                senders_stake_addr: strake_address,
                outputs,
                inputs,
                excludes,
                collateral,
                network,
                current_slot: curr_slot,
            })
        } else {
            Err(MurinError::new(&format!(
                "Error the provided string '{src}' cannot be parsed into 'TxData' "
            )))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CBORTransaction {
    r#type: String,
    description: String,
    #[serde(rename = "camelCase")]
    cbor_hex: String,
}

// Customer Payout
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CPO {
    po_id: i64,
    pw: String,
}

impl CPO {
    pub fn new(po_id: i64, pw: String) -> Self {
        CPO { po_id, pw }
    }

    pub fn get_po_id(&self) -> i64 {
        self.po_id
    }

    pub fn get_pw(&self) -> String {
        self.pw.to_owned()
    }
}

pub async fn find_token_utxos(
    inputs: TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
) -> Result<TransactionUnspentOutputs, MurinError> {
    let mut out = TransactionUnspentOutputs::new();
    if !inputs.is_empty() && !assets.is_empty() {
        for asset in assets {
            let mut needed_amt = asset.2;
            for i in 0..inputs.len() {
                let unspent_output = inputs.get(i);
                let value = unspent_output.output().amount();
                if let Some(multi) = value.multiasset() {
                    if let Some(toks) = multi.get(&asset.0) {
                        if let Some(amt) = toks.get(&asset.1) {
                            // for tok in 0..toks.len()  {
                            if needed_amt >= asset.2 {
                                log::debug!(
                                    "Found a utxo containing {} tokens {}.{}!",
                                    asset.2.to_str(),
                                    hex::encode(asset.0.to_bytes()),
                                    hex::encode(asset.1.to_bytes())
                                );
                                if !out.contains_tx(&unspent_output) {
                                    out.add(&unspent_output);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(MurinError::new(
            "cannot find token utxos , one of the provided inputs is empty",
        ));
    }
    Ok(out)
}

pub fn find_token_utxos_na(
    inputs: &TransactionUnspentOutputs,
    assets: Vec<TokenAsset>,
    on_addr: Option<&caddr::Address>,
) -> Result<TransactionUnspentOutputs, MurinError> {
    let mut out = TransactionUnspentOutputs::new();
    let ins = inputs.clone();
    if !inputs.is_empty() && !assets.is_empty() {
        for asset in assets.clone() {
            let mut needed_amt = asset.2;
            debug!("Set Needed Amount: {:?}", needed_amt);
            for i in 0..ins.len() {
                let unspent_output = ins.get(i);
                if let Some(addr) = on_addr {
                    if unspent_output.output().address().to_bytes() != addr.to_bytes() {
                        continue;
                    }
                };
                let value = unspent_output.output().amount();
                if let Some(multi) = value.multiasset() {
                    if let Some(toks) = multi.get(&asset.0) {
                        if let Some(amt) = toks.get(&asset.1) {
                            if needed_amt.compare(&cutils::to_bignum(0)) > 0 {
                                log::debug!(
                                    "Found a utxo containing {} tokens {}.{}!",
                                    asset.2.to_str(),
                                    hex::encode(asset.0.to_bytes()),
                                    hex::encode(asset.1.to_bytes())
                                );
                                if !out.contains_tx(&unspent_output) {
                                    out.add(&unspent_output);
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                } else {
                                    needed_amt = needed_amt.clamped_sub(&amt);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(MurinError::new(
            "ERROR: cannot find token utxos , one of the provided inputs is empty",
        ));
    }

    if out.is_empty() {
        debug!("Inputs: {:?}", inputs);
        return Err(MurinError::new(
            "ERROR: The token is not available in the utxo set",
        ));
    }

    out.optimize_on_assets(assets)?;
    Ok(out)
}

pub fn input_selection(
    token_utxos: Option<&TransactionUnspentOutputs>,
    needed_value: &mut cutils::Value,
    txins: &TransactionUnspentOutputs,
    collateral: Option<cutils::TransactionUnspentOutput>,
    on_addr: Option<&caddr::Address>,
) -> Result<(clib::TransactionInputs, TransactionUnspentOutputs), MurinError> {
    debug!("\n\nMULTIASSETS: {:?}\n\n", txins);

    let (mut purecoinassets, mut multiassets) =
        crate::cardano::supporting_functions::splitt_coin_multi(txins);

    let mut nv = needed_value.clone();
    let mut selection = TransactionUnspentOutputs::new();
    let mut acc = cutils::Value::new(&cutils::to_bignum(0u64));
    let mut txins = clib::TransactionInputs::new();

    let overhead = 50u64;

    if let Some(token_utxos) = token_utxos {
        for i in 0..token_utxos.len() {
            selection.add(&token_utxos.get(i));
            acc = acc
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            nv = nv
                .checked_add(&token_utxos.get(i).output().amount())
                .unwrap();
            debug!("\n\nAdded Script Utxo to Acc Value : \n {:?}\n", acc);
            // Delete script input from multi assets
            if let Some(i) = multiassets.find_utxo_index(&token_utxos.get(i)) {
                let tutxo = multiassets.swap_remove(i);
                debug!(
                    "Deleted token utxo from multiasset inputs: \n {:?}\n",
                    tutxo
                );
            }
        }
    }

    if let Some(cutxo) = collateral {
        debug!("Col: {:?}", cutxo);
        let c_index = crate::cardano::supporting_functions::find_collateral_by_txhash_txix(
            &cutxo,
            &purecoinassets,
        );
        debug!(
            "Some collateral to check for deletion found, Index: {:?}",
            c_index
        );
        if let Some(index) = c_index {
            let col = purecoinassets.swap_remove(index);
            debug!("Deleted collateral from inputs: {:?}\n", col);
            // Double check
            if crate::cardano::supporting_functions::find_collateral_by_txhash_txix(
                &cutxo,
                &purecoinassets,
            )
            .is_some()
            {
                return Err(MurinError::new(
                    "PANIC COLLATERAL COULDN'T BE EXCLUDED FROM SELECTION SET",
                ));
            }
        }
    }

    // lookup tokens from needed value
    let mut tokens_to_find = crate::cardano::models::Tokens::new();
    if needed_value.multiasset().is_some() {
        if needed_value.multiasset().unwrap().len() > 0 {
            let pids = needed_value.multiasset().unwrap().keys();
            for i in 0..pids.len() {
                let policy = pids.get(i);
                let assets = needed_value.multiasset().unwrap().get(&policy);
                if let Some(a) = assets {
                    let assetnames = a.keys();
                    for j in 0..assetnames.len() {
                        let assetname = assetnames.get(j);
                        if let Some(amount) = a.get(&assetname) {
                            tokens_to_find.push((policy.clone(), assetname, amount));
                        }
                    }
                }
            }
        }
        let token_selection = find_token_utxos_na(&multiassets.clone(), tokens_to_find, on_addr)?;
        if !token_selection.is_empty() {
            for i in 0..token_selection.len() {
                selection.add(&token_selection.get(i));
                acc = acc
                    .checked_add(&token_selection.get(i).output().amount())
                    .unwrap();
                debug!("\n\nAdded Script Utxo to Acc Value : \n {:?}\n", acc);
                // Delete script input from multi assets
                if let Some(i) = multiassets.find_utxo_index(&token_selection.get(i)) {
                    let tutxo = multiassets.swap_remove(i);
                    debug!(
                        "Deleted token utxo from multiasset inputs: \n {:?}\n",
                        tutxo
                    );
                }
            }
        }
    }

    multiassets.sort_by_coin();
    purecoinassets.sort_by_coin();

    debug!("\n\nMULTIASSETS: {:?}\n\n", multiassets);
    debug!("\n\npurecoinassets: {:?}\n\n", purecoinassets);

    let utxo_count = multiassets.len() + purecoinassets.len();
    let mut max_run = 0;
    debug!("\n\nNV: {:?}", nv);
    debug!("\n\nNV: {:?}", acc);
    debug!(
        "\nbefore while! Utxo Count: {:?}, {:?} \n",
        utxo_count,
        (nv.coin().compare(&acc.coin()) > 0)
    );
    while nv.coin().compare(&acc.coin()) > 0 && max_run < utxo_count {
        nv = nv.checked_sub(&acc).unwrap();

        if purecoinassets.is_empty() {
            // Find the tokens we want in the multis
            debug!("\nWe look for multiassets!\n");
            let ret = crate::cardano::supporting_functions::find_suitable_coins(
                &mut nv,
                &mut multiassets,
                overhead,
            );
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&cutils::to_bignum(ret.1)).unwrap());
                }
                None => {
                    //ToDo: Do not panic -> Error
                    panic!("ERROR: Not enough input utxos available to balance the transaction");
                }
            }
            let _ = multiassets.pop();
        } else {
            // Fine enough Ada to pay the transaction
            let ret = crate::cardano::supporting_functions::find_suitable_coins(
                &mut nv,
                &mut purecoinassets,
                overhead,
            );
            debug!("Return coinassets: {:?}", ret);
            match ret.0 {
                Some(utxos) => {
                    for u in utxos {
                        selection.add(&u);
                    }
                    acc.set_coin(&acc.coin().checked_add(&cutils::to_bignum(ret.1)).unwrap());
                    debug!("\nSelection in coinassets: {:?}", selection);
                    debug!("\nAcc in coinassets: {:?}", acc);
                }
                None => {
                    return Err(MurinError::new(
                        "ERROR: Not enough input utxos available to balance the transaction",
                    ));
                }
            }
            let _ = purecoinassets.pop();
        }
        max_run += 1;
    }
    for txuo in selection.clone() {
        txins.add(&txuo.input());
    }
    debug!("\n\nSelection: {:?}\n\n", selection);
    Ok((txins, selection))
}

#[derive(Clone, PartialEq, Eq)]
pub struct PaymentValue {
    payer: caddr::Address,
    value: cutils::Value,
}

impl PaymentValue {
    pub fn new(payer: &caddr::Address, value: &cutils::Value) -> Self {
        PaymentValue {
            payer: payer.clone(),
            value: value.clone(),
        }
    }

    pub fn get_payer(&self) -> caddr::Address {
        self.payer.clone()
    }

    pub fn get_value(&self) -> cutils::Value {
        self.value.clone()
    }

    pub fn set_payer(&mut self, a: &caddr::Address) {
        self.payer = a.clone();
    }

    pub fn set_value(&mut self, v: &cutils::Value) {
        self.value = v.clone();
    }
}

#[derive(Clone)]
pub struct Persona {
    pub txuo_out: Option<clib::TransactionOutput>,
    pub own_address: caddr::Address,
    pub stake_key: ccrypto::Ed25519KeyHash,
    pub change: cutils::Value,
    pub receive: Vec<PaymentValue>,
    pub used_inputs: TransactionUnspentOutputs,
}

impl PartialEq for Persona {
    fn eq(&self, other: &Self) -> bool {
        let txout = if let Some(o_txuo_out) = &other.txuo_out {
            if let Some(s_txuo_out) = &self.txuo_out {
                o_txuo_out.to_bytes() == s_txuo_out.to_bytes()
            } else {
                false
            }
        } else {
            false
        };

        txout
            && self.own_address == other.own_address
            && self.stake_key == other.stake_key
            && self.change.to_bytes() == other.change.to_bytes()
            && self.receive == other.receive
            && self.used_inputs.to_hex() == other.used_inputs.to_hex()
    }
}

impl Persona {
    pub fn new(own_address: &caddr::Address, receive: &[PaymentValue]) -> Self {
        Persona {
            txuo_out: None,
            own_address: own_address.clone(),
            stake_key: crate::cardano::get_stake_address(own_address),
            change: cutils::Value::new(&cutils::to_bignum(0)),
            receive: receive.to_owned(),
            used_inputs: TransactionUnspentOutputs::new(),
        }
    }

    fn ceq(&self, other: &Self) -> bool {
        self.stake_key == other.stake_key
    }

    fn add_left(&mut self, other: &Self) -> Result<(), MurinError> {
        if let Some(o_txuo_out) = &other.txuo_out {
            if let Some(s_txuo_out) = &self.txuo_out {
                let new_val = s_txuo_out.amount().checked_add(&o_txuo_out.amount())?;
                self.txuo_out = Some(clib::TransactionOutput::new(
                    &s_txuo_out.address(),
                    &new_val,
                ));
            } else {
                self.txuo_out = Some(o_txuo_out.clone())
            }
        }

        self.change = self.change.checked_add(&other.change)?;
        self.receive.extend(other.receive.iter().cloned());
        self.used_inputs.merge(other.used_inputs.to_owned());

        Ok(())
    }

    pub fn addl(&mut self, other: &Self) -> Result<(), MurinError> {
        if !self.ceq(other) {
            return Err(MurinError::new(
                "Payment Combination Error: The two Persona do not are not the same",
            ));
        }
        self.add_left(other)?;
        Ok(())
    }

    // returns Zero-Value if no output is set
    pub fn get_output_value(&self) -> cutils::Value {
        if let Some(value) = &self.txuo_out {
            value.amount()
        } else {
            cutils::Value::new(&cutils::to_bignum(0))
        }
    }

    // returns 0 if no output is set
    pub fn get_output_coins(&self) -> cutils::BigNum {
        if let Some(value) = &self.txuo_out {
            value.amount().coin()
        } else {
            cutils::to_bignum(0)
        }
    }

    pub fn get_min_utxo_value(&self) -> cutils::BigNum {
        if let Some(txo) = &self.txuo_out {
            calc_min_ada_for_utxo(&self.get_output_value(), txo.data_hash())
        } else {
            cutils::to_bignum(0)
        }
    }

    pub fn get_change_value(&self) -> cutils::Value {
        self.change.clone()
    }

    pub fn get_change_coins(&self) -> cutils::BigNum {
        self.change.coin()
    }

    pub fn get_change_min_utxo_value(&self) -> cutils::BigNum {
        calc_min_ada_for_utxo(&self.get_change_value(), None)
    }

    pub fn set_output_value(
        &mut self,
        v: cutils::Value,
        hash: Option<ccrypto::DataHash>,
    ) -> Result<(), MurinError> {
        let mut new_output = clib::TransactionOutput::new(&self.own_address, &v);
        if let Some(h) = hash {
            new_output.set_data_hash(&h);
        }
        self.txuo_out = Some(new_output);

        Ok(())
    }

    pub fn add_to_output_value(&mut self, v: cutils::Value) -> Result<(), MurinError> {
        let new_val: cutils::Value;
        if let Some(txo) = &self.txuo_out {
            new_val = txo.amount().checked_add(&v)?;
            if let Some(h) = txo.data_hash() {
                self.set_output_value(new_val, Some(h))?;
            } else {
                self.set_output_value(new_val, None)?;
            }
        } else {
            self.set_output_value(v, None)?;
        }
        Ok(())
    }

    pub fn set_output_coins(&mut self, c: cutils::BigNum) -> Result<(), MurinError> {
        let mut new_val = cutils::Value::new(&c);
        if let Some(txo) = &self.txuo_out {
            new_val = txo.amount().checked_add(&new_val)?;
            if let Some(h) = txo.data_hash() {
                self.set_output_value(new_val, Some(h))?;
            } else {
                self.set_output_value(new_val, None)?;
            }
        } else {
            self.set_output_value(new_val, None)?;
        }
        Ok(())
    }

    pub fn set_min_utxo_value(&mut self) -> Option<cutils::BigNum> {
        let c_min = self.get_min_utxo_value();
        if self.get_output_coins().compare(&c_min) == (-1) {
            self.set_output_coins(c_min)
                .expect("Could not set coin value in set min utxo value for persona");
            return Some(c_min);
        }
        None
    }

    pub fn add_change(&mut self, v: &cutils::Value) -> Result<(), MurinError> {
        self.change = self.change.checked_add(v)?;
        Ok(())
    }

    pub fn txo_from_payers(&mut self) -> Result<(), MurinError> {
        let mut new_val = cutils::Value::new(&cutils::to_bignum(0));
        for payer in &self.receive {
            new_val = new_val.checked_add(&payer.value)?;
        }
        self.txuo_out = Some(clib::TransactionOutput::new(&self.own_address, &new_val));
        let min_utxo = self.get_min_utxo_value();

        if new_val.coin().compare(&min_utxo) == -1 {
            self.set_min_utxo_value();
            self.receive.push(PaymentValue::new(
                &self.own_address,
                &cutils::Value::new(&min_utxo),
            ))
        }
        Ok(())
    }

    pub fn find_own_inputs(
        &mut self,
        avail_input_utxos: &mut TransactionUnspentOutputs,
        selected_input_utxos: &mut TransactionUnspentOutputs,
    ) -> Result<(), MurinError> {
        for payer in &self.receive {
            let needed = payer.get_value();
            let mut input_utxos = avail_input_utxos.clone();
            match needed.multiasset() {
                Some(_) => {
                    let assets = crate::value_to_tokens(&needed)?;
                    let mut utxo_selection = find_token_utxos_na(
                        avail_input_utxos,
                        assets,
                        Some(payer.get_payer()).as_ref(),
                    )?;

                    input_utxos.delete_set(&utxo_selection);

                    select_coins(
                        &mut utxo_selection,
                        &mut input_utxos,
                        &needed,
                        &payer.get_payer(),
                        &self.own_address,
                    )?;
                }
                None => {
                    let mut utxo_selection = TransactionUnspentOutputs::new();
                    select_coins(
                        &mut utxo_selection,
                        &mut input_utxos,
                        &needed,
                        &payer.get_payer(),
                        &self.own_address,
                    )?;
                }
            }
            self.used_inputs = input_utxos.clone();
            selected_input_utxos.merge(input_utxos);
        }
        Ok(())
    }
}

// Helper function for select coins
// Recursive apply minutxo until enough Ada is available
fn select_min_utxo_input_coins(
    paying_address: &caddr::Address,
    needed: &cutils::Value,
    selected_value: &mut cutils::Value,
    selected_inputs: &mut TransactionUnspentOutputs,
    avail_input_utxos: &mut TransactionUnspentOutputs,
) -> Result<(), MurinError> {
    let diff = selected_value.checked_sub(needed)?;

    let change_min_utxo = calc_min_ada_for_utxo(&diff, None);

    if diff.coin().compare(&change_min_utxo) == -1 {
        let missing_coins = change_min_utxo.checked_sub(&diff.coin())?;
        let additional_utxos =
            avail_input_utxos.coin_value_subset_minutxo(&missing_coins, paying_address);
        avail_input_utxos.delete_set(&additional_utxos);
        selected_inputs.merge(additional_utxos);
        *selected_value = selected_inputs.calc_total_value()?;
        return select_min_utxo_input_coins(
            paying_address,
            needed,
            selected_value,
            selected_inputs,
            avail_input_utxos,
        );
    }
    Ok(())
}

pub fn select_coins(
    utxo_selection: &mut TransactionUnspentOutputs,
    input_utxos: &mut TransactionUnspentOutputs,
    needed: &cutils::Value,
    payer: &caddr::Address,
    own_address: &caddr::Address,
) -> Result<(), MurinError> {
    let mut selected_value = utxo_selection.calc_total_value()?;

    if selected_value.coin().compare(&needed.coin()) == -1 {
        // Not enough Ada we need more
        let missing_coins = needed.coin().checked_sub(&selected_value.coin())?;
        let additional_inputs = input_utxos.coin_value_subset_minutxo(&missing_coins, payer);
        input_utxos.delete_set(&additional_inputs);
        utxo_selection.merge(additional_inputs);

        // make sure enough Ada is available
        select_min_utxo_input_coins(
            own_address,
            needed,
            &mut selected_value,
            utxo_selection,
            input_utxos,
        )?;
    }
    Ok(())
}

pub struct Personas {
    pub p: Vec<Persona>,
}

impl Personas {
    pub fn combine(&mut self) -> Result<(), MurinError> {
        let mut blist = self.p.clone();
        Personas::combinator(&mut self.p, &mut blist);
        Ok(())
    }

    fn combinator<'a>(
        a: &'a mut Vec<Persona>,
        b: &'a mut Vec<Persona>,
    ) -> (&'a Vec<Persona>, &'a Vec<Persona>) {
        match b[..] {
            [] => (a, b),

            _ => {
                b.retain(|n| *n != a[0]);
                let combinable_bs: Vec<_> = b
                    .iter_mut()
                    .filter(|n| a[0].ceq(n))
                    .map(|n| n.to_owned())
                    .collect();
                combinable_bs.iter().for_each(|n| a[0].add_left(n).unwrap());
                b.retain(|n| !combinable_bs.contains(n));
                a.retain(|n| !combinable_bs.contains(n));
                let len = a.len() - 1;
                a.swap(0, len);

                Personas::combinator(a, b)
            }
        }
    }
}

pub fn balance_transaction(
    inputs_txuos: &mut TransactionUnspentOutputs,
    personas: &mut Personas,
) -> Result<(clib::TransactionOutputs, clib::TransactionInputs), MurinError> {
    // Combine Payments if paying_address and txuo_out destination address (stake address without script addresses) are equal
    personas.combine()?;

    let mut invalid: bool = false;
    // Set min Utxo for all outputs
    personas
        .p
        .iter_mut()
        .for_each(|n| match n.txo_from_payers() {
            Ok(_) => {}
            Err(_) => invalid = true,
        });

    if invalid {
        return Err(MurinError::new("Error in buidling initial txos"));
    }

    // Select input utxos
    let mut global_selection_txuos = TransactionUnspentOutputs::new();

    personas.p.iter_mut().for_each(|n| {
        match n.find_own_inputs(inputs_txuos, &mut global_selection_txuos) {
            Ok(_) => {}
            Err(e) => {
                info!("Error in closure to find inputs: {:?}", e.to_string());
                invalid = true;
            }
        }
    });

    if invalid {
        return Err(MurinError::new("Error in finding inputs for payments"));
    }

    // ToDo:
    // calcualte change for each persona
    // Idee:
    // für jede Persona gehe druch "receive" suche die utxos des payers in "used inputs" bzw. "globaler selection" mittels stake_key
    // in der globalen selection ziehen wir den "value" aus "receive" von dem value in gloablen utxo ab. Wir suchen das selbe Utxo in
    // used inputs und ziehen dort den restwert des gloablen Utxos ab, wir sollten den "value" wert aus receive erhalten.
    // die "globale selection" enthält nun alle übriggebliebenen Werte.
    // wir gehen durch die "globale selection" und schauen ob es Persona gibt die über den stake_key des utxos identifiziert werden können
    // ist das der Fall wird der Wert des Utxos dem "change" value der Persona gutgeschrieben. Kann keine Persona zugeordnet werden,
    // so geht der Restwert des Utxos zurück an die Adresse im Utxo, hierzu wird eine neue Persona hinzugefügt und der Wert change gutgeschrieben.
    // Kann die minUtxo bedingung nit erfüllt werden, bricht der Transactionbau ab.

    // In einem letzen Schritt addieren wir change und output wert im output,
    // theoretisch sollte der Wert aufgrund der selektionsbedingungen über dem minUtxo Wert liegen.

    /*
        personas.p.iter_mut().for_each(|n| {
            let change : Vec<_> = avail_change.0.iter().filter(|m| m.output().address() == n.txuo_out.output().address()).collect();

            // Add change to the existing output
            for c in change {
                match n.add_to_output_value(c.output().amount()){
                    Ok(_) => {},
                    Err(e) => {
                        info!("Error in closure to add change to outputs: {:?}",e.to_string());
                        invalid = true;
                    }
                };
                let mut del = TransactionUnspentOutputs::new();
                del.add(c);
                avail_change.to_owned().delete_set(&del);

            }
        });

        if invalid {
            return Err(MurinError::new("Error in closure to add change to outputs"))
        }
    */

    // Ab jetzt können wir nurnoch neue Utxos wählen
    inputs_txuos.delete_set(&global_selection_txuos);

    let mut txouts = clib::TransactionOutputs::new();
    let mut txins = clib::TransactionInputs::new();

    // Check and handle to big utxos
    let to_big: Vec<_> = personas
        .p
        .iter()
        .filter(|n| n.txuo_out.clone().unwrap().to_bytes().len() > 5000)
        .collect();

    if !to_big.is_empty() {
        // Split the outputs which are to big in half if necessary add another input to get more Ada
        personas.p.to_owned().retain(|n| !to_big.contains(&n));

        for p in to_big {
            let splits = half_utxo(&p.txuo_out.clone().unwrap(), inputs_txuos, &p.own_address);
            for i in 0..splits.0.len() {
                txouts.add(&splits.0.get(i));
            }
            for elem in splits.1 {
                txins.add(&elem.input());
            }
        }
    }

    // Make TxIns and TxOuts
    for elem in personas.p.iter().cloned() {
        txouts.add(&elem.txuo_out.unwrap());
        for u in elem.used_inputs {
            txins.add(&u.input());
        }
    }

    Ok((txouts, txins))
}

pub fn half_utxo(
    v: &clib::TransactionOutput,
    inputs: &mut TransactionUnspentOutputs,
    paying_address: &caddr::Address,
) -> (clib::TransactionOutputs, TransactionUnspentOutputs) {
    let mut one = cutils::Value::new(&cutils::to_bignum(0));
    let mut two = v.amount();
    let mut out = clib::TransactionOutputs::new();
    let mut used_inputs = TransactionUnspentOutputs::new();

    let size_two = two.to_bytes().len();
    let mut multiasset_one = clib::MultiAsset::new();
    let mut multiasset_two = two
        .multiasset()
        .expect("Error: The TxOut to be halfed has no multiassets!");
    while one.to_bytes().len() < size_two / 2 {
        let mut temp = clib::MultiAsset::new();
        let policys = multiasset_two.keys();
        if let Some(assets) = multiasset_two.get(&policys.get(0)) {
            multiasset_one.insert(&policys.get(0), &assets);
            temp.insert(&policys.get(0), &assets);
        };
        multiasset_two = multiasset_two.sub(&temp);
    }
    one.set_multiasset(&multiasset_one);
    let min_utxo_one = calc_min_ada_for_utxo(&one, None);
    two.set_multiasset(&multiasset_two);
    let min_utxo_two = calc_min_ada_for_utxo(&two, None);

    let total_min_ada = min_utxo_one.checked_add(&min_utxo_two).unwrap();
    match two.coin().compare(&total_min_ada) {
        -1 => {
            // Not enough Ada for both min Utxos
            let missing_coins = total_min_ada.clamped_sub(&two.coin());
            let additional_inputs =
                inputs.coin_value_subset_minutxo(&missing_coins, paying_address);
            inputs.delete_set(&additional_inputs);
            used_inputs.merge(additional_inputs.clone());

            two.set_coin(&min_utxo_two);
            one.set_coin(&min_utxo_one);
            out.add(&clib::TransactionOutput::new(&v.address(), &one));
            out.add(&clib::TransactionOutput::new(&v.address(), &two));

            let mut tot_val = additional_inputs.calc_total_value().unwrap();
            tot_val = tot_val
                .checked_sub(&cutils::Value::new(&total_min_ada))
                .unwrap();
            let new_change = clib::TransactionOutput::new(paying_address, &tot_val);
            if new_change.to_bytes().len() > 5000 {
                let next = half_utxo(&new_change, inputs, paying_address);
                for i in 0..next.0.len() {
                    out.add(&next.0.get(i));
                }
                inputs.merge(next.1);
            } else {
                out.add(&new_change);
            }
        }
        _ => {
            // Enough Ada
            two.set_coin(&two.coin().clamped_sub(&min_utxo_one));
            one.set_coin(&min_utxo_one);

            out.add(&clib::TransactionOutput::new(&v.address(), &one));
            out.add(&clib::TransactionOutput::new(&v.address(), &two));
        }
    }

    (out, used_inputs)
}

pub fn find_assets_in_value(
    v: &cutils::Value,
    a: &Vec<TokenAsset>,
) -> (bool, cutils::Value, cutils::Value) {
    let mut new_ma = clib::MultiAsset::new();
    let mut rest = clib::MultiAsset::new();
    let coin = v.coin();

    let mut new_val = cutils::Value::new(&cutils::to_bignum(0));
    let mut rest_val = cutils::Value::new(&cutils::to_bignum(0));

    let mut flag = false;

    if v.multiasset().is_none() {
        return (flag, new_val, rest_val);
    }
    let ma = v.multiasset().unwrap();
    for t in a {
        let amt = &ma.get_asset(&t.0, &t.1);
        if cutils::from_bignum(amt) > 0 {
            flag = true;
            let mut ramt = *amt;
            if amt.compare(&t.2) > 0 {
                ramt = amt.clamped_sub(&t.2);
            }

            let mut assets = clib::Assets::new();
            assets.insert(&t.1, &t.2);
            new_ma.insert(&t.0, &assets);

            let mut rassets = clib::Assets::new();
            rassets.insert(&t.1, &ramt);
            rest.insert(&t.0, &rassets);
        } else {
            let mut assets = clib::Assets::new();
            assets.insert(&t.1, &t.2);
            rest.insert(&t.0, &assets);
        }
    }

    new_val.set_multiasset(&new_ma);
    rest_val.set_multiasset(&rest);

    rest_val.set_coin(&coin);

    (flag, new_val, rest_val)
}

pub fn calc_min_ada_for_utxo(
    value: &cutils::Value,
    dh: Option<ccrypto::DataHash>,
) -> cutils::BigNum {
    //utxoEntrySize (txout) * coinsPerUTxOWord
    //utxoEntrySize (txout) = utxoEntrySizeWithoutVal + size (v) + dataHashSize (dh)
    let dhsize: u64 = match dh {
        Some(_) => 10u64, //(datumhash.to_bytes().len())  as u64
        None => 0u64,
    };

    let size = bundle_size(
        value,
        &models::OutputSizeConstants {
            k0: 2,
            k1: 6,
            k2: 12,
            k3: 1,
            _k4: 8,
        },
    );

    let utxo_entry_size_without_val = 27u64; //29
    let min_ada = cutils::to_bignum(dhsize + utxo_entry_size_without_val + size as u64)
        .checked_mul(&cutils::to_bignum(34482u64))
        .unwrap(); //(value.to_bytes().len() as u64)) / 2
    debug!(
        "\nCalculated MinAda: {:?} for Value: {:?}\n",
        min_ada, value
    );

    //Limit max Val size
    let val_size = value.to_bytes().len();
    debug!("ValueSize: {:?}", val_size);
    if val_size > 5000 {
        //
        // ToDO: Panic is no options we need to split up the values and check this before arriving here.
        //
        //return Err(MurinError::new("ERROR: exceeded max value size"));
        panic!("exceeded max value size ")
    }

    min_ada
}

pub fn min_ada_for_utxo(output_: &TransactionOutput) -> Result<TransactionOutput, MurinError> {
    let mut output: TransactionOutput = output_.clone();
    let pppath = std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH")?;
    let coins_per_byte = crate::pparams::ProtocolParameters::read_protocol_parameter(&pppath)?;
    for _ in 0..3 {
        let required_coin = to_bignum(output.to_bytes().len() as u64)
            .checked_add(&to_bignum(160))?
            .checked_mul(&to_bignum(coins_per_byte.utxo_cost_per_byte))?;
        if output.amount().coin().less_than(&required_coin) {
            let mut v = output.amount().clone();
            v.set_coin(&required_coin);
            output = TransactionOutput::new(&output.address(), &v);
            if let Some(dh) = output_.data_hash() {
                output.set_data_hash(&dh)
            }
            if let Some(p) = output_.plutus_data() {
                output.set_plutus_data(&p)
            }
            if let Some(sref) = output_.script_ref() {
                output.set_script_ref(&sref)
            }
        } else {
            return Ok(output);
        }
    }
    let mut v = output.amount();
    v.set_coin(&to_bignum(u64::MAX));
    output = TransactionOutput::new(&output.address(), &v);
    if let Some(dh) = output_.data_hash() {
        output.set_data_hash(&dh)
    }
    if let Some(p) = output_.plutus_data() {
        output.set_plutus_data(&p)
    }
    if let Some(sref) = output_.script_ref() {
        output.set_script_ref(&sref)
    }
    min_ada_for_utxo(&output)
}

pub fn bundle_size(value: &cutils::Value, osc: &models::OutputSizeConstants) -> usize {
    match &value.multiasset() {
        Some(assets) => {
            //Anzahl Tokenss
            let mut num_assets: usize = 0;
            //AssetName Length
            let mut anl: usize = 0;
            // PolicyId Length
            let mut pil: usize = 0;

            let policy_ids = assets.keys();

            for policy in 0..policy_ids.len() {
                let pid = &policy_ids.get(policy);
                pil += pid.to_bytes().len();
                num_assets += assets.get(pid).unwrap().len();
                let tns = assets.get(&policy_ids.get(policy)).unwrap().keys();
                for tn in 0..tns.len() {
                    anl += tns.get(tn).name().len();
                }
            }

            fn roundup_bytes_to_words(b: usize) -> usize {
                quot(b + 7, 8)
            }

            osc.k1 + (roundup_bytes_to_words((num_assets * osc.k2) + anl + (osc.k3 * pil)))
        }

        None => osc.k0,
    }
}

pub fn quot<T>(a: T, b: T) -> T
where
    T: Sub<Output = T> + Rem<Output = T> + Div<Output = T> + Copy + Clone + std::fmt::Display,
{
    (a - (a % b)) / b
}

pub async fn create_and_submit_cbor_tx(tx: String, tx_hash: String) -> Result<String, MurinError> {
    let cli_tx = CBORTransaction {
        r#type: "Tx BabbageEra".to_string(),
        description: "drasil build this transaction for you".to_string(),
        cbor_hex: tx,
    };
    debug!("{:?}", cli_tx);
    let node_tx_hash = submit_tx(&cli_tx, &tx_hash).await?;

    Ok(node_tx_hash)
}

pub async fn submit_endpoint(
    tx: &[u8],
    endpoint: String,
    own_tx_hash: &String,
    client: &reqwest::Client,
) -> Result<(String, String, bool), MurinError> {
    use futures::pin_mut;

    let response = client
        .post(endpoint.clone())
        .header("Content-Type", "application/cbor")
        .body(tx.to_owned())
        .send();
    pin_mut!(response);

    match tokio::time::timeout(std::time::Duration::from_secs(5), &mut response).await {
        Err(_) => {
            debug!("Taking more than five seconds");
            Ok((
                "".to_string(),
                format!("ERROR: '{endpoint:?}' is not available"),
                false,
            ))
        }
        Ok(no_timeout) => match no_timeout {
            Ok(resp) => {
                let mut err = String::new();
                let mut txhash = String::new();
                info!("Response: {:?}", resp);
                let r_status = resp.status();
                let resp_text = resp.text().await?;
                if r_status != http::StatusCode::ACCEPTED {
                    err = format!("ERROR on tx submission: {resp_text:?}");
                    debug!("Error, Endpoint: {} : {:?}", endpoint, err);
                } else {
                    txhash = resp_text.replace('\"', "");
                }
                let assert = *own_tx_hash == txhash;

                Ok((txhash, err, assert))
            }
            Err(e) => Ok((
                "".to_string(),
                format!("ERROR: '{e:?}' is not available"),
                false,
            )),
        },
    }
}

pub async fn submit_tx(tx: &CBORTransaction, own_tx_hash: &String) -> Result<String, MurinError> {
    let submit1 = std::env::var("TX_SUBMIT_ENDPOINT1")?;
    let submit2 = std::env::var("TX_SUBMIT_ENDPOINT2")?;
    let submit3 = std::env::var("TX_SUBMIT_ENDPOINT3")?;

    let client = reqwest::Client::new();
    let tx = hex::decode(tx.cbor_hex.clone())?;
    let mut response1 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit1, own_tx_hash, &client).await {
        Ok(x) => response1 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    let mut response2 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit2, own_tx_hash, &client).await {
        Ok(x) => response2 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    let mut response3 = (String::new(), String::new(), false);
    match submit_endpoint(&tx, submit3, own_tx_hash, &client).await {
        Ok(x) => response3 = x,
        Err(e) => {
            error!("Error: '{}'", e.to_string())
        }
    };

    if response1.2 || response2.2 || response3.2 {
        Ok(own_tx_hash.clone())
    } else {
        Err(MurinError::new(
            &(response1.1 + &response2.1 + &response3.1),
        ))
    }
}

pub fn harden(num: u32) -> u32 {
    0x80000000 + num
}

pub fn get_input_position(
    inputs: clib::TransactionInputs,
    elem: TransactionUnspentOutput,
) -> (usize, Vec<ccrypto::TransactionHash>) {
    let mut index: usize;
    let mut my_index = Vec::<ccrypto::TransactionHash>::new();
    for i in 0..inputs.len() {
        debug!("Script Input: {:?} at position : {:?}\n", inputs.get(i), i);
        my_index.push(inputs.get(i).transaction_id());
        if inputs.get(i).transaction_id() == elem.input().transaction_id()
            && inputs.get(i).index() == elem.input().index()
        {
            index = i;
            debug!(
                "Found Script Input: {:?} at position : {:?}\n",
                inputs.get(i),
                index
            );
        }
    }

    debug!("\nUnsortiert: {:?}", my_index);
    my_index.sort();
    debug!("\nSortiert: {:?}", my_index);
    let index = my_index
        .iter()
        .enumerate()
        .find(|&r| r.1 == &elem.input().transaction_id())
        .unwrap()
        .0;
    debug!("\nIndex: {:?}\n", index);

    (index, my_index)
}

pub fn split_value(
    value: cutils::Value,
) -> Result<(Vec<cutils::Value>, Option<cutils::BigNum>), MurinError> {
    let coins = value.coin();
    let mut val_coins = cutils::to_bignum(0);
    let val_tok = models::value_to_tokens(&value)?;
    let mut values = Vec::<cutils::Value>::new();
    for tok in val_tok {
        let mut value = models::tokens_to_value(&[tok].to_vec());
        let min_utxo_val = super::calc_min_ada_for_utxo(&value, None);
        val_coins = val_coins.checked_add(&min_utxo_val)?;
        value.set_coin(&min_utxo_val);
        values.push(value);
    }

    match coins.compare(&val_coins) {
        k if k > 0 => {
            let c = values[0].coin();
            let diff = coins.checked_sub(&val_coins)?;
            values[0].set_coin(&c.checked_add(&diff)?);
            Ok((values, None))
        }
        k if k < 0 => {
            // More Ada needed
            let diff = val_coins.checked_sub(&coins)?;
            Ok((values, Some(diff)))
        }
        _ => Ok((values, None)),
    }
}

pub fn minimize_coins_on_values(
    values: Vec<cutils::Value>,
) -> Result<Vec<cutils::Value>, MurinError> {
    let mut out = Vec::<cutils::Value>::new();
    let ada = values.iter().fold(
        cutils::Value::new(&cutils::to_bignum(0)),
        |mut acc: cutils::Value, x: &cutils::Value| {
            let c = x.coin();
            let mut xc = x.clone();
            let muv = calc_min_ada_for_utxo(x, None);
            if c.compare(&muv) > 0 {
                let diff = c.checked_sub(&muv).unwrap();
                xc.set_coin(&muv);
                acc.set_coin(&acc.coin().checked_add(&diff).unwrap());
                out.push(xc);
                acc
            } else {
                out.push(xc);
                acc
            }
        },
    );
    if cutils::from_bignum(&ada.coin()) > 0 {
        out.push(ada);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceFees {
    pub fee: BigNum,
    pub fee_addr: Address,
}
