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

use murin::clib::address::Address;
use murin::TransactionUnspentOutputs;

use murin::modules::transfer::models::{TransWallet, TransWallets};
use murin::stdtx::build_wallet_asset_transfer::{AtSATBuilder, AtSATParams};

use murin::{b_decode_addr, PerformTxb};

// Handler for ordinary token transfers
pub(crate) async fn handle_stx(bss: &BuildStdTx) -> crate::Result<String> {
    match bss
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::StdTx {
            wallet_addresses,
            transfers,
        } => {
            let err = Err(CmdError::Custom {
                str: format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'",
                    bss.transaction_pattern().operation()
                ),
            }
            .into());
            if transfers.is_empty() {
                return err;
            }

            if let Some(addresses) = wallet_addresses {
                for addr in addresses.iter() {
                    b_decode_addr(addr).await?;
                }
            }
        }
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong data provided for '{:?}'", bss.tx_type()),
            }
            .into());
        }
    }
    log::debug!("Checks okay...");

    let mut bsstp = bss.transaction_pattern().clone();

    log::debug!("Try to create raw data...");
    let std_asset_txd = bss
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_stdassettx()
        .await?;

    let addresses =
        std_asset_txd
            .wallet_addresses
            .iter()
            .fold(Vec::<String>::new(), |mut acc, n| {
                acc.push(n.to_bech32(None).unwrap());
                acc
            });
    bsstp.set_used_addresses(&addresses);

    log::debug!("Try to create raw data2...");
    let mut gtxd = bsstp.into_txdata().await?;
    log::debug!("Try to create raw data3...");
    if !std_asset_txd.wallet_addresses.is_empty() {
        let wallet_utxos = std_asset_txd.wallet_addresses.iter().fold(
            TransactionUnspentOutputs::new(),
            |mut acc, n| {
                acc.merge(mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap());
                acc
            },
        );
        gtxd.set_inputs(wallet_utxos);

        // ToDo: go through all addresses and check all stake keys are equal
        let sa = murin::get_reward_address(&std_asset_txd.wallet_addresses[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(std_asset_txd.wallet_addresses.clone());
    }

    log::debug!("Try to determine slot...");
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

    let mut wallets = TransWallets::new();

    let first_address_str =
        mimir::select_addr_of_first_transaction(&gtxd.get_stake_address().to_bech32(None)?)?;
    let first_addr = Address::from_bech32(&first_address_str)?;

    // ToDo:
    // - Add Wallets

    // If addresses are provided check they all belong to the same wallet, if yes
    // get utxos for the addresses and build TransWallet with that UTxOs

    let uw = TransWallet::new(&first_addr, &gtxd.get_inputs());
    wallets.add_wallet(&uw);

    // - Add Endpoint to get AssetHandles From AddressSet

    let txb_param: AtSATParams = (&std_asset_txd, &wallets, &first_addr);
    let asset_transfer = AtSATBuilder::new(txb_param);
    let builder = murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&asset_transfer).await;

    if let Err(err) = &bld_tx {
        return Err(CmdError::Custom {
            str: err.to_string(),
        }
        .into());
    }
    let bld_tx = bld_tx?;

    log::debug!("Try to create raw tx...");
    let tx = murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &std_asset_txd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bss.customer_id()),
        &[],
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bss.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}
