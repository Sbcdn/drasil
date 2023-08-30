use crate::datamodel::Operation;
use crate::protocol::create_response;
use crate::BuildStdTx;
use crate::CmdError;
use drasil_murin::address_from_string_non_async;
use drasil_murin::clib;
use drasil_murin::PerformTxb;
use drasil_murin::TransactionUnspentOutputs;

pub(crate) async fn handle_stake_delegation(bst: &BuildStdTx) -> crate::Result<String> {
    match bst
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No transaction specific data supplied for stake delegation")?
    {
        Operation::StakeDelegation { .. } => (),
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong input data provided for '{:?}'", bst.tx_type()),
            }
            .into());
        }
    }
    let op = &bst.transaction_pattern().operation().unwrap();
    let (mut delegtxd, addresses) = match op {
        Operation::StakeDelegation {
            poolhash: _,
            addresses,
        } => (op.into_stake_delegation().await?, addresses),
        _ => {
            return Err(CmdError::Custom {
                str: format!("ERROR wrong input data provided for '{:?}'", bst.tx_type()),
            }
            .into())
        }
    };
    // intotxdata only works with the transaction pattern, we also need to make the address pattern acceptable

    let wal_addr = if let Some(addr) = addresses {
        addr
            .iter()
            .fold(Vec::<clib::address::Address>::new(), |mut acc, a| {
                acc.push(
                    address_from_string_non_async(a).unwrap()
                );
                acc
            })
    } else {
        vec![]
    };
    let addresses = wal_addr.iter().fold(Vec::<String>::new(), |mut acc, n| {
        acc.push(
            n.to_bech32(None).unwrap()
        );
        acc
    });
    debug!("stake delegation addresses: {:?}", addresses);
    let mut bst_tmp = bst.transaction_pattern().clone();
    bst_tmp.set_used_addresses(&addresses[..]);
    debug!(
        "bst.transaction_pattern().used_addresses: {:?}",
        bst_tmp.used_addresses()
    );
    let mut gtxd = bst_tmp.into_txdata().await?;
    gtxd.set_user_id(bst.customer_id());

    if !wal_addr.is_empty() {
        let wallet_utxos = wal_addr
            .iter()
            .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                acc.merge(
                    drasil_mimir::get_address_utxos(
                        &n.to_bech32(None).unwrap()
                    ).unwrap()
                );
                acc
            });
        gtxd.set_inputs(wallet_utxos);

        // ToDo: go through all addresses and check all stake keys are equal
        let sa = drasil_murin::reward_address_from_address(&wal_addr[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(wal_addr.clone());
    }

    log::debug!("Try to determine slot...");
    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(CmdError::Custom {
                str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
            }
            .into());
        }
    };
    let slot = drasil_mimir::get_slot(&mut dbsync)?;
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

    let registered = drasil_mimir::check_stakeaddr_registered(&bech32_stake_addr)?;
    delegtxd.set_registered(Some(registered));

    log::debug!("Try to build transaction...");

    let txb_param: drasil_murin::txbuilder::delegation::AtDelegParams = &delegtxd;
    let deleg = drasil_murin::txbuilder::delegation::AtDelegBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &Vec::<String>::new());
    let bld_tx = builder.build(&deleg).await?;

    info!("Build Successful!");
    let tx = drasil_murin::utxomngr::RawTx::new(
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

#[cfg(test)]
mod tests {
    use crate::{BuildStdTx, StdTxType, TransactionPattern, Operation};
    use tokio;
    use std::env::set_var;

    #[tokio::test]
    async fn handle_stake_delegation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        set_var("REDIS_DB", "redis://127.0.0.1:6379/0");
        set_var("REDIS_DB_URL_UTXOMIND", "redis://127.0.0.1:6379/0");
        set_var("REDIS_CLUSTER", "false");
        let customer_id = 0;
        let txtype = StdTxType::DelegateStake;
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t".to_string();
        let addr1 = "addr_test1qp8cprhse9pnnv7f4l3n6pj0afq2hjm6f7r2205dz0583egaeu9dhacmtx94652q4ym0v9v2mcra0n28d5lrtjqzsgxqgk5t8s".to_string();
        let addresses = Some(vec![addr1]);
        let script_spec = Operation::StakeDelegation { poolhash, addresses };
        let network = 0;
        let txpattern = TransactionPattern::new_empty(customer_id, &script_spec, network);
        let bst = BuildStdTx::new(customer_id, txtype, txpattern);
        let func_value = super::handle_stake_delegation(&bst).await?; // might change with time

        let real_value = "310737567dbedda0d4e9780861bad5ade7bd00fb14032302b1b7bd9c|84a50081825820395eab6c60ec00faeff30391c683551119669b0aeb8c778948afbe83299b29b1010181825839004f808ef0c94339b3c9afe33d064fea40abcb7a4f86a53e8d13e878e51dcf0adbf71b598b5d5140a936f6158ade07d7cd476d3e35c802820c1b000000e8d4a26cfb021a0002a305031a01905d4b048183028200581c1dcf0adbf71b598b5d5140a936f6158ade07d7cd476d3e35c802820c581c0ae25c559d7f7f2c22759007c7caeb740a0a3a47e3cd3ec976458a8fa0f5f6".to_string();
        log::info!("handle_stake_delegation real value example: {}", "310737567dbedda0d4e9780861bad5ade7bd00fb14032302b1b7bd9c|84a50081825820395eab6c60ec00faeff30391c683551119669b0aeb8c778948afbe83299b29b1010181825839004f808ef0c94339b3c9afe33d064fea40abcb7a4f86a53e8d13e878e51dcf0adbf71b598b5d5140a936f6158ade07d7cd476d3e35c802820c1b000000e8d4a26cfb021a0002a305031a01905d4b048183028200581c1dcf0adbf71b598b5d5140a936f6158ade07d7cd476d3e35c802820c581c0ae25c559d7f7f2c22759007c7caeb740a0a3a47e3cd3ec976458a8fa0f5f6");
        log::info!("handle_stake_delegation func value (changes with time): {}", func_value);

        assert_eq!(func_value.len(), real_value.len());

        Ok(())
    }
}