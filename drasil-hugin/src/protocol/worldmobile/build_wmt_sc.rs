//! WorldMobile token staking smart contract.

use drasil_murin::worldmobile::models::StakeTxData;
use drasil_murin::{wallet, TransactionUnspentOutputs};

use crate::database::TBContracts;
use crate::datamodel::staking::StakingAction;
use crate::datamodel::{ContractAction, Operation};
use crate::{BuildContract, CmdError};

/// Handle WMT staking operations.
async fn handle_wmt_staking(build_contract: BuildContract) -> crate::Result<String> {
    let operation = build_contract
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?;
    let Operation::WmtStaking {
        amount: staking_amount,
        target_en,
    } = operation
    else {
        return Err(Box::new(CmdError::Custom {
            str: String::from("Unexpected opreation"),
        }));
    };
    let ContractAction::StakingAction(action) = build_contract.action() else {
        return Err(Box::new(CmdError::Custom {
            str: String::from("Unexpected opreation"),
        }));
    };

    let mut gtxd = build_contract.transaction_pattern().into_txdata().await?;
    let sender_addresses = gtxd.get_senders_addresses();

    if !sender_addresses.is_empty() {
        let wallet_utxos =
            sender_addresses
                .iter()
                .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                    acc.merge(
                        drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap(),
                    );
                    acc
                });
        gtxd.set_inputs(wallet_utxos);

        // ToDo: go through all addresses and check all stake keys are equal
        let sa = wallet::reward_address_from_address(&sender_addresses[0])?;
        gtxd.set_stake_address(sa);
        gtxd.set_senders_addresses(sender_addresses.clone());
    }
    // Get the first address which is the unique address to identify a wallet.
    // This is the first address where the staking key was used.
    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&gtxd.get_stake_address().to_hex())?,
    )
    .await?;

    let stake_data = StakeTxData {
        staking_amount,
        registration_datum,
        wallet_addr,
        registration_utxos,
    };

    let mptxd = build_contract
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_mp(gtxd.clone().get_inputs())
        .await?;
    gtxd.set_user_id(build_contract.customer_id as i64);
    let mut dbsync = drasil_mimir::establish_connection()?;
    let slot = drasil_mimir::get_slot(&mut dbsync)?;
    gtxd.set_current_slot(slot as u64);

    let ret: String;
    match action {
        StakingAction::Stake => {
            let contract = TBContracts::get_active_contract_for_user(
                build_contract.customer_id as i64,
                build_contract.ctype.to_string(),
                None,
            )?;

            let sc_addr = contract.address.to_string();
            let sc_version = contract.version.to_string();

            let mut dbsync = drasil_mimir::establish_connection()?;
            let slot = drasil_mimir::get_slot(&mut dbsync)?;
            gtxd.set_current_slot(slot as u64);
            todo!()
            // let res = build_mp_listing(&gtxd, &mptxd, &sc_addr, &sc_version).await?;
            // let tx = drasil_murin::utxomngr::RawTx::new(
            //     &res.get_tx_body(),
            //     &res.get_txwitness(),
            //     &res.get_tx_unsigned(),
            //     &res.get_metadata(),
            //     &gtxd.to_string(),
            //     &mptxd.to_string(),
            //     &res.get_used_utxos(),
            //     &hex::encode(gtxd.get_stake_address().to_bytes()),
            //     &(build_contract.customer_id as i64),
            //     &[contract.contract_id],
            // );
            // ret = super::create_response(
            //     &res,
            //     &tx,
            //     build_contract.transaction_pattern().wallet_type().as_ref(),
            // )?
            // .to_string();
        }
        StakingAction::UnStake => todo!(),
    }
    // Ok(ret)
}
