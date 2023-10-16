use drasil_murin::{RawTx, PerformTxb};

use crate::{create_response, BuildStdTx, Operation, CmdError};

pub(crate) async fn handle_reward_withdrawal(bst: &BuildStdTx) -> crate::Result<String> {
    let op = bst
        .transaction_pattern()
        .operation()
        .filter(|op| op.is_reward_withdrawal())
        .ok_or("ERROR: No transaction specific data supplied for Ada withdrawal")?;

    let mut gtxd = bst
        .transaction_pattern()
        .clone()
        .into_txdata()
        .await?;
    gtxd.set_user_id(bst.customer_id());

    let mut dbsync = drasil_mimir::establish_connection().map_err(|e| 
        CmdError::Custom {str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string())}
    )?;

    gtxd.set_current_slot(drasil_mimir::get_slot(&mut dbsync)? as u64);
    let withdraw_txd = op.into_withdrawal().await?;

    let bld_tx = &drasil_murin::TxBuilder::new(
            &gtxd,
            &Vec::<String>::new()
        )
        .build(
            &drasil_murin::txbuilder::stdtx::AtAWBuilder::new(
                &match op {
                    Operation::RewardWithdrawal{} => withdraw_txd,
                    _ => {
                        return Err(CmdError::Custom {
                            str: format!("ERROR wrong input data provided for '{:?}'", bst.tx_type()),
                        }
                        .into())
                    }
                }
            )
        )
        .await?;

    Ok(
        create_response(
            &bld_tx,
            &RawTx::new(
                &bld_tx.get_tx_body(), 
                &bld_tx.get_txwitness(), 
                &bld_tx.get_tx_unsigned(), 
                &bld_tx.get_metadata(), 
                &gtxd.to_string(), 
                &"".to_string(), 
                &bld_tx.get_used_utxos(), 
                &hex::encode(gtxd.get_stake_address().to_bytes()), 
                &bst.customer_id(), 
                &[-1]
            ),
            bst.transaction_pattern().wallet_type().as_ref(),
        )?
        .to_string()
    )
}