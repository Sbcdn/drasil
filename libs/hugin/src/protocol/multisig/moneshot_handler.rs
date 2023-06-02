/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::OneShotReturn;
use crate::drasildb::TBContracts;
use crate::BuildMultiSig;
use crate::CmdError;
use murin::PerformTxb;

use serde_json::json;

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
    let mut dbsync = match mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
            }
            .into());
        }
    };
    log::debug!("Get Slot...");
    let slot = match mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!(
                    "ERROR could not determine current slot: '{:?}'",
                    e.to_string()
                ),
            }
            .into());
        }
    };
    gtxd.set_current_slot(slot as u64);

    log::debug!("Create Oneshot policy...");
    log::debug!("Current Slot: {:?}", slot);
    let oneshotwallet = murin::wallet::create_wallet();
    let oneshotpolicy = murin::minter::create_onshot_policy(&oneshotwallet.3, slot as u64);

    log::debug!("Check contract...");
    let contract = TBContracts::get_liquidity_wallet(&bms.customer_id())?;
    log::debug!("Try to determine additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )?;
    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &contract.address,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
    let ns_script = oneshotpolicy.0;

    //ToDO:
    //
    // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd

    log::debug!("Set utxos for input...");
    gtxd.set_inputs(mimir::get_address_utxos(&contract.address)?);

    log::debug!("Try to build transactions...");
    let txb_param: murin::txbuilders::minter::build_oneshot_mint::AtOSMParams = (
        murin::cip30::b_decode_addr(&contract.address).await?,
        ns_script.clone(),
        &minttxd,
    );
    let minter = murin::txbuilders::minter::build_oneshot_mint::AtOSMBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &pkvs);
    let bld_tx = match builder.build(&minter).await {
        Ok(o) => o,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not build transaction: '{:?}'", e.to_string()),
            }
            .into());
        }
    };

    log::debug!("Try to create Raw Tx...");
    let tx = murin::utxomngr::RawTx::new(
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
    let txh = murin::finalize_rwd(
        &hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()),
        tx,
        vec![hex::encode(oneshotwallet.0.as_bytes())],
    )
    .await?;

    log::debug!("Store used utxos...");
    murin::utxomngr::usedutxos::store_used_utxos(
        &txh,
        &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
    )?;

    let mut tokennames = Vec::<String>::new();
    let mut amounts = Vec::<u64>::new();
    let policy_id = hex::encode(ns_script.hash().to_bytes());

    for t in minttxd.get_mint_tokens() {
        tokennames.push(hex::encode(t.1.name()));
        amounts.push(murin::clib::utils::from_bignum(&t.2));
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
