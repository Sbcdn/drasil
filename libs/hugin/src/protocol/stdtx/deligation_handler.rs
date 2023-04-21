/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::Operation;
use crate::protocol::create_response;
use crate::BuildStdTx;
use crate::CmdError;
use murin::PerformTxb;

pub(crate) async fn handle_stake_delegation(bst: &BuildStdTx) -> crate::Result<String> {
    match bst
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::StakeDelegation { .. } => (),
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong data provided for '{:?}'", bst.tx_type()),
            }
            .into());
        }
    }

    let mut delegtxd = bst
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_stake_delegation()
        .await?;
    let mut gtxd = bst.transaction_pattern().into_txdata().await?;
    gtxd.set_user_id(bst.customer_id());

    let mut dbsync = mimir::establish_connection()?;
    let slot = mimir::get_slot(&mut dbsync)?;
    gtxd.set_current_slot(slot as u64);

    let bech32_stake_addr = match gtxd.get_stake_address().to_bech32(None) {
        Ok(ba) => ba,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("Could not convert Stake Address;' {e:?}'"),
            }
            .into());
        }
    };

    let registered = mimir::check_stakeaddr_registered(&bech32_stake_addr)?;
    delegtxd.set_registered(Some(registered));

    log::debug!("Try to build transaction...");

    let txb_param: murin::txbuilders::delegation::AtDelegParams = &delegtxd;
    let deleg = murin::txbuilders::delegation::AtDelegBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &Vec::<String>::new());
    let bld_tx = builder.build(&deleg).await?;

    info!("Build Successful!");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &delegtxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bst.customer_id()),
        &[(-1)],
    );
    debug!("RAWTX data: {:?}", tx);

    let ret = create_response(
        &bld_tx,
        &tx,
        bst.transaction_pattern().wallet_type().as_ref(),
    )?;

    Ok(ret.to_string())
}
