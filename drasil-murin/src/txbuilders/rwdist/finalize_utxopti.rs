use super::*;
use crate::txmind::RawTx;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{crypto as ccrypto, utils as cutils};

pub async fn finalize_utxopti(raw_tx: RawTx, pvks: Vec<String>) -> Result<String, MurinError> {
    info!("Start building final transaction");
    let tx_body = clib::TransactionBody::from_bytes(hex::decode(raw_tx.get_txbody())?)?;
    let mut tx_witness =
        clib::TransactionWitnessSet::from_bytes(hex::decode(raw_tx.get_txwitness())?)?;
    let tx_hash = hex::encode(cutils::hash_transaction(&tx_body).to_bytes());

    let root_key1 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pvks[0])?)?;
    let account_key1 = root_key1
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let prv1 = account_key1.to_raw_key(); // for signatures

    let vkwitness_1 = cutils::make_vkey_witness(&cutils::hash_transaction(&tx_body), &prv1);

    let root_key2 = clib::crypto::Bip32PrivateKey::from_bytes(&hex::decode(&pvks[1])?)?;
    let account_key2 = root_key2
        .derive(harden(1852u32))
        .derive(harden(1815u32))
        .derive(harden(0u32));
    let prv2 = account_key2.to_raw_key(); // for signatures

    let vkwitness_2 = cutils::make_vkey_witness(&cutils::hash_transaction(&tx_body), &prv2);

    let mut tx_witness_all_vkeys = ccrypto::Vkeywitnesses::new();
    tx_witness_all_vkeys.add(&vkwitness_1);
    tx_witness_all_vkeys.add(&vkwitness_2);

    tx_witness.set_vkeys(&tx_witness_all_vkeys);

    debug!("TxWitness: {:?}", hex::encode(tx_witness.to_bytes()));

    let fin_tx = hex::encode(clib::Transaction::new(&tx_body, &tx_witness, None).to_bytes());

    create_and_submit_cbor_tx(fin_tx, tx_hash).await
}
