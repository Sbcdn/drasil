use super::*;
use crate::txmind::RawTx;
use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;


/// The finalization step applies a signature to a stored already built transaction
/// and submits it to the cardano network afterwards.
/// It will restore a stored tx witness, which makes it possible to apply a signature from the system already
/// during the build step without exposing it to the requesting user. 
pub async fn finalize(signature: &String, raw_tx: RawTx) -> Result<String, MurinError> {
    info!("Specific Raw Data: {:?}", raw_tx.get_tx_specific_rawdata());
    
    let tx_witness_signature = clib::TransactionWitnessSet::from_bytes(hex::decode(signature)?)?;

    info!("Start building final transaction");
    debug!("{:?}", raw_tx.get_txwitness());

    let tx_aux = clib::metadata::AuxiliaryData::from_bytes(hex::decode(raw_tx.get_txaux())?)?;

    //Check if aux Data is empty, if yes set it None in the final tx
    let aux_data = if tx_aux.native_scripts().is_none()
        && tx_aux.plutus_scripts().is_none()
        && tx_aux.metadata().is_none()
    {
        None
    } else {
        Some(tx_aux.clone())
    };

    let tx_body = clib::TransactionBody::from_bytes(hex::decode(raw_tx.get_txbody())?)?;

    let mut tx_witness_stored =
        clib::TransactionWitnessSet::from_bytes(hex::decode(raw_tx.get_txwitness())?)?;

    let vkeys = tx_witness_signature.vkeys().unwrap();
    tx_witness_stored.set_vkeys(&vkeys);
    let fin_tx =
        hex::encode(clib::Transaction::new(&tx_body, &tx_witness_stored, aux_data).to_bytes());
    let tx_hash = hex::encode(cutils::hash_transaction(&tx_body).to_bytes());
    create_and_submit_cbor_tx(fin_tx, tx_hash).await
}
