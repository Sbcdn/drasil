/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::BuildMultiSig;
use crate::CmdError;
use murin::PerformTxb;

pub(crate) async fn handle_customer_payout(bms: &BuildMultiSig) -> crate::Result<String> {
    info!("verify transaction data...");
    // ToDo:
    // Verify there is a unhandled payout existing for this user with the security code passed in cpo_data,
    // Payout need to be verified and approved by a DrasilAdmin (In the best case after creation and signature of the customer)
    // The Drasil verification would apply the last needed MultiSig Key for the payout so no accidential payout is possible.

    info!("create raw data...");
    let cpo_data = bms
        .transaction_pattern()
        .script()
        .unwrap()
        .into_cpo()
        .await?;
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;

    info!("establish database connections...");
    let mut drasildbcon = crate::database::drasildb::establish_connection()?;

    let contract = crate::drasildb::TBContracts::get_contract_uid_cid(
        cpo_data.get_user_id(),
        cpo_data.get_contract_id(),
    )?;

    let _contract_address = murin::address::Address::from_bech32(&contract.address).unwrap();

    info!("retrieve additional data...");
    let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
        &mut drasildbcon,
        &contract.contract_id,
        &contract.user_id,
        &contract.version,
    )?;
    info!("Drasil Connection!");
    info!("keyloc: {:?}", keyloc);

    let ns_script = contract.plutus.clone();
    let _ns_version = contract.version.to_string();

    let mut dbsync = match mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
            }
            .into());
        }
    };
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

    // ToDO:
    // Determine Available Payout Sum and write it into cpo_data so the txbuild can create correct transaction
    // The Sum is determined automatically by: Outputsum = Ada_Available_on_contract - (Total_Liquidity)
    // make sure no tokens are leaving the contract (possibly a rearrangement of Utxos is needed before and after the payout?)

    // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
    // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
    let contract_utxos = mimir::get_address_utxos(&mut dbsync, &contract.address)?;

    gtxd.set_inputs(contract_utxos);

    let ident = crate::encryption::mident(
        &contract.user_id,
        &contract.contract_id,
        &contract.version,
        &contract.address,
    );
    let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;

    log::debug!("Try to build transaction...");

    let txb_param: murin::txbuilders::stdtx::build_cpo::AtCPOParams = (
        //Vec<(caddr::Address, cutils::Value, Option<TransactionUnspentOutputs>)>
        // ToDo: Payout Values
        Vec::<(
            &murin::clib::address::Address,
            &murin::clib::utils::Value,
            Option<&murin::TransactionUnspentOutputs>,
        )>::new(),
        murin::clib::NativeScript::from_bytes(hex::decode(ns_script)?).map_err::<CmdError, _>(
            |_| CmdError::Custom {
                str: "could not convert string to native script".to_string(),
            },
        )?,
    );
    let cpo = murin::txbuilders::stdtx::build_cpo::AtCPOBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &pkvs);
    let _bld_tx = builder.build(&cpo).await?;

    /*

    info!("Build Successful!");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &minttxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id as i64),
        &contract.contract_id,
        &contract.version,
    );
    debug!("RAWTX data: {:?}",tx);
    let used_utxos = tx.get_usedutxos().clone();
    let txh = murin::finalize_rwd(&hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()), tx, keyloc.pvks).await?;
    murin::utxomngr::usedutxos::store_used_utxos(&txh, &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?)?;

    let ret = super::create_response(&bld_tx, &tx, bms.transaction_pattern().wallet_type().as_ref())?;
    */
    let ret = "Not implemented";
    Ok(ret.to_string())
}
