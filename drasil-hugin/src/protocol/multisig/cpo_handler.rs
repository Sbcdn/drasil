use drasil_murin::MurinError;
use drasil_murin::{cardano, PerformTxb, TransactionUnspentOutputs, TxData};

use crate::admin::get_vaddr;
use crate::BuildMultiSig;
use crate::CaValue;
use crate::TBCaPayment;
use crate::TBCaPaymentHash;
use crate::TBContracts;
use crate::TBDrasilUser;

pub(crate) async fn handle_customer_payout(bms: &BuildMultiSig) -> crate::Result<String> {
    let poid = bms
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_cpo()
        .await?;
    let po = TBCaPayment::find(&poid.get_po_id()).map_err(|e| MurinError::Custom(e.to_string()))?;

    if po.stauts_pa == "cancel" || po.stauts_bl.is_some() {
        return Err("ERROR payout is invalid".into());
    }

    log::debug!("Verify password...");
    TBDrasilUser::verify_pw_userid(&po.user_id, &poid.get_pw()).map_err(|e| MurinError::Custom(e.to_string()))?;

    if po.stauts_bl.is_some() {
        return Err("ERROR this payout was processed before".into());
    }

    log::debug!("Try to connect to drasil db and get user...");
    let user = TBDrasilUser::get_user_by_user_id(&po.user_id).map_err(|e| MurinError::Custom(e.to_string()))?;

    //Trigger Build and submit payout transaction
    let contract = TBContracts::get_contract_uid_cid(po.user_id, po.contract_id).map_err(|e| MurinError::Custom(e.to_string()))?;

    log::debug!("Generating TxData...");
    let mut gtxd = TxData::new(
        Some(vec![po.contract_id]),
        vec![drasil_murin::wallet::address_from_string(&get_vaddr(&po.user_id).await.map_err(|e| MurinError::Custom(e.to_string()))?).await?],
        None,
        TransactionUnspentOutputs::new(),
        cardano::get_network_from_address(&contract.address)?,
        0,
    )?;

    log::debug!("Try to determine verified address...");
    let verified_addr = gtxd.get_senders_address(None).unwrap();

    log::debug!("Make payout values...");
    let cv = serde_json::from_str::<CaValue>(&po.value)?.into_cvalue().map_err(|e| MurinError::Custom(e.to_string()))?;
    let txo_values = vec![(&verified_addr, &cv, None)];
    log::debug!("Try to build transaction...");
    // ToDo: Check that payout sum cannot spent liquidity

    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!(
                "ERROR could not connect to dbsync: '{:?}'",
                e.to_string()).into())
        }
    };
    let slot = match drasil_mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                "ERROR could not determine current slot: '{:?}'",
                e.to_string()).into())
        }
    };
    gtxd.set_current_slot(slot as u64);
    log::info!("DB Sync Slot: {}", slot);

    let utxos = drasil_mimir::get_address_utxos(&contract.address)
        .expect("MimirError: cannot find address utxos");
    gtxd.set_inputs(utxos);

    log::debug!("Try to determine additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    ).map_err(|e| MurinError::Custom(e.to_string()))?;

    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &contract.address,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;

    log::debug!("Try to build transaction...");

    let txb_param: drasil_murin::txbuilder::stdtx::build_cpo::AtCPOParams = (
        txo_values,
        drasil_murin::clib::NativeScript::from_bytes(hex::decode(&contract.plutus)?)
            .map_err::<crate::CmdError, _>(|_| crate::CmdError::Custom {
            str: "could not convert string to native script".to_string(),
        }).map_err(|e| MurinError::Custom(e.to_string()))?,
    );
    let cpo = drasil_murin::txbuilder::stdtx::build_cpo::AtCPOBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &pkvs);
    let bld_tx = builder.build(&cpo).await?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &"CPayout".to_string(),
        &bld_tx.get_used_utxos(),
        &"".to_string(),
        &user.user_id,
        &[contract.contract_id],
    );
    trace!("RAWTX data: {:?}", tx);

    let used_utxos = tx.get_usedutxos().clone();
    let txh = drasil_murin::finalize_rwd(
        &hex::encode(&drasil_murin::clib::TransactionWitnessSet::new().to_bytes()),
        tx,
        pkvs,
    )
    .await?;
    drasil_murin::utxomngr::usedutxos::store_used_utxos(
        &txh,
        &drasil_murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
    )?;

    // On Success update status
    let result = po.update_txhash(&txh).await.map_err(|e| MurinError::Custom(e.to_string()))?;
    TBCaPaymentHash::create(&result).await.map_err(|e| MurinError::Custom(e.to_string()))?;
    Ok(serde_json::json!(result).to_string())
}
