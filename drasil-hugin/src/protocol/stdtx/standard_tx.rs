use crate::datamodel::Operation;
use crate::protocol::create_response;
use crate::BuildStdTx;

use drasil_murin::clib::address::Address;
use drasil_murin::MurinError;
use drasil_murin::TransactionUnspentOutputs;

use drasil_murin::modules::transfer::models::{TransWallet, TransWallets};
use drasil_murin::stdtx::build_wallet_asset_transfer::{AtSATBuilder, AtSATParams};

use drasil_murin::{wallet, PerformTxb};

// Handler for ordinary token transfers
pub(crate) async fn handle_stx(bss: &BuildStdTx) -> Result<String, MurinError> {
    match bss
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")
        .map_err(|e| {
            MurinError::new(&format!(
                "Could not get 'Operation' from transaction patttern, error: {:?}",
                e.to_string()
            ))
        })? {
        Operation::StdTx {
            wallet_addresses,
            transfers,
        } => {
            let err = Err(MurinError::new(&format!(
                "ERROR wrong data provided for script specific parameters: '{:?}'",
                bss.transaction_pattern().operation()
            )));
            if transfers.is_empty() {
                return err;
            }

            if let Some(addresses) = wallet_addresses {
                for addr in addresses.iter() {
                    wallet::address_from_string(addr).await?;
                }
            }
        }
        _ => {
            return Err(MurinError::new(&format!(
                "ERROR wrong data provided for '{:?}'",
                bss.tx_type()
            )))
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
                acc.merge(drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap());
                acc
            },
        );
        gtxd.set_inputs(wallet_utxos);

        let sa = wallet::reward_address_from_address(&std_asset_txd.wallet_addresses[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(std_asset_txd.wallet_addresses.clone());
    }

    log::debug!("Try to determine slot...");
    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(MurinError::new(&format!(
                "ERROR could not connect to dbsync: '{:?}'",
                e.to_string()
            )));
        }
    };
    let slot = match drasil_mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(MurinError::new(&format!(
                "ERROR could not determine current slot: '{:?}'",
                e.to_string()
            )))
        }
    };
    gtxd.set_current_slot(slot as u64);

    let mut wallets = TransWallets::new();

    let first_address_str =
        drasil_mimir::select_addr_of_first_transaction(&gtxd.get_stake_address().to_bech32(None)?)
            .map_err(|e| {
                MurinError::new(&format!(
                    "Could not get address of first transaction: {:?}",
                    e.to_string()
                ))
            })?;
    let first_addr = Address::from_bech32(&first_address_str)?;

    let uw = TransWallet::new(&first_addr, &gtxd.get_inputs());
    wallets.add_wallet(&uw);

    let txb_param: AtSATParams = (&std_asset_txd, &wallets, &first_addr);
    let asset_transfer = AtSATBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&asset_transfer).await;

    if let Err(err) = &bld_tx {
        return Err(MurinError::new(&err.to_string()));
    }
    let bld_tx = bld_tx?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
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
