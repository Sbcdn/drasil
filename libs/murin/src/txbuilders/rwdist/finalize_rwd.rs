/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use super::*;
use crate::txmind::RawTx;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{crypto as ccrypto, utils as cutils};
use clib::crypto::Vkeywitnesses;

pub async fn finalize_rwd(
    signature: &String,
    raw_tx: RawTx,
    pvks: Vec<String>,
) -> Result<String, MurinError> {
    let tx_witness_signature = clib::TransactionWitnessSet::from_bytes(hex::decode(signature)?)?;

    info!("Start building final transaction");
    debug!("{:?}", raw_tx.get_txwitness());

    let tx_aux = clib::metadata::AuxiliaryData::from_bytes(hex::decode(raw_tx.get_txaux())?)?;

    //Check if aux Data is empty, if yes set it None in the final tx
    let mut aux_data = Some(tx_aux.clone());
    if tx_aux.native_scripts() == None
        && tx_aux.plutus_scripts() == None
        && tx_aux.metadata() == None
    {
        aux_data = None
    };

    let tx_body = clib::TransactionBody::from_bytes(hex::decode(raw_tx.get_txbody())?)?;
    let tx_hash = hex::encode(cutils::hash_transaction(&tx_body).to_bytes());
    let mut tx_witness_stored =
        clib::TransactionWitnessSet::from_bytes(hex::decode(raw_tx.get_txwitness())?)?;

    let mut vkeys_signature = Vkeywitnesses::new();
    if let Some(vks) = tx_witness_signature.vkeys() {
        vkeys_signature = vks;
    };

    let mut tx_witness_all_vkeys = ccrypto::Vkeywitnesses::new();
    if let Some(vkeys) = tx_witness_stored.vkeys() {
        tx_witness_all_vkeys = vkeys
    };

    for i in 0..vkeys_signature.len() {
        tx_witness_all_vkeys.add(&vkeys_signature.get(i))
    }

    for pv in pvks {
        let root_key = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pv)?)?;
        let account_key = root_key
            .derive(harden(1852u32))
            .derive(harden(1815u32))
            .derive(harden(0u32));
        let prv = account_key.to_raw_key(); // for signatures
        let vkwitness = cutils::make_vkey_witness(&cutils::hash_transaction(&tx_body), &prv);
        tx_witness_all_vkeys.add(&vkwitness);
    }

    tx_witness_stored.set_vkeys(&tx_witness_all_vkeys);

    debug!("TxWitness: {:?}", hex::encode(tx_witness_stored.to_bytes()));

    let fin_tx =
        hex::encode(clib::Transaction::new(&tx_body, &tx_witness_stored, aux_data).to_bytes());

    create_and_submit_cbor_tx(fin_tx, tx_hash).await
}
