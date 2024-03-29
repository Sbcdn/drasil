use drasil_murin::MurinError;
use drasil_murin::{wallet, PerformTxb};
use serde_json::json;

use crate::datamodel::OneShotReturn;
use crate::drasildb::TBContracts;
use crate::BuildMultiSig;

pub(crate) async fn handle_onehshot_mint(bms: &BuildMultiSig) -> crate::Result<String> {
    log::debug!("Entered Oneshot Minter...");
    let minttxd = bms
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_mintdata()
        .await?;
    log::debug!("Minter Txd: {:?}", minttxd);
    let mut txp = bms.transaction_pattern();
    txp.set_used_addresses(&[minttxd.get_payment_addr_bech32()?]);
    log::debug!("Transaction Patter: {:?}\n", &txp);
    log::debug!("Try to create general transaction data...");
    let mut gtxd = txp.into_txdata().await?;
    log::debug!("General TX Data: {:?}", gtxd);
    log::debug!("Connect to dbsync...");
    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()).into());
        }
    };
    log::debug!("Get Slot...");
    let slot = match drasil_mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                "ERROR could not determine current slot: '{:?}'",
                e.to_string()
            )
            .into());
        }
    };
    gtxd.set_current_slot(slot as u64);

    log::debug!("Create Oneshot policy...");
    log::debug!("Current Slot: {:?}", slot);
    let oneshotwallet = drasil_murin::wallet::create_wallet();
    let oneshotpolicy = drasil_murin::minter::create_onshot_policy(&oneshotwallet.3, slot as u64);

    log::debug!("Check contract...");
    let contract = TBContracts::get_liquidity_wallet(&bms.customer_id())
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    log::debug!("Try to determine additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )
    .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &contract.address,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
    let ns_script = oneshotpolicy.0;

    log::debug!("Set utxos for input...");
    gtxd.set_inputs(
        drasil_mimir::get_address_utxos(&contract.address)
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?,
    );

    log::debug!("Try to build transactions...");
    let txb_param: drasil_murin::txbuilder::minter::build_oneshot_mint::AtOSMParams = (
        wallet::address_from_string(&contract.address).await?,
        ns_script.clone(),
        &minttxd,
    );
    let minter = drasil_murin::txbuilder::minter::build_oneshot_mint::AtOSMBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &pkvs);
    let bld_tx = match builder.build(&minter).await {
        Ok(o) => o,
        Err(e) => {
            return Err(format!("ERROR could not build transaction: '{:?}'", e.to_string()).into());
        }
    };

    log::debug!("Try to create Raw Tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &minttxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id()),
        &[(-1)],
    );

    log::debug!("Finalize...");
    let used_utxos = tx.get_usedutxos().clone();
    let txh = drasil_murin::finalize_rwd(
        &hex::encode(&drasil_murin::clib::TransactionWitnessSet::new().to_bytes()),
        tx,
        vec![hex::encode(oneshotwallet.0.as_bytes())],
    )
    .await?;

    log::debug!("Store used utxos...");
    drasil_murin::utxomngr::usedutxos::store_used_utxos(
        &txh,
        &drasil_murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
    )?;

    let mut tokennames = Vec::<String>::new();
    let mut amounts = Vec::<u64>::new();
    let policy_id = hex::encode(ns_script.hash().to_bytes());

    for t in minttxd.get_mint_tokens() {
        tokennames.push(hex::encode(t.1.name()));
        amounts.push(drasil_murin::clib::utils::from_bignum(&t.2));
    }

    let ret = OneShotReturn::new(
        &policy_id,
        &tokennames,
        &amounts,
        &txh,
        &bld_tx.get_metadata(),
    );

    Ok(json!(ret).to_string())
}
