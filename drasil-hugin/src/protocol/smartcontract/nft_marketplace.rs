//pub(crate) mod reward_handler;
//pub(crate) use reward_handler::handle_rewardclaim;
// use drasil_murin::cardano::MIN_ADA;

// use crate::database::TBContracts;
// use crate::datamodel::{ContractAction, MarketplaceActions, Operation, TransactionPattern};
use crate::BuildContract; //, CmdError};
pub async fn handle_marketplace(_build_contract: BuildContract) -> crate::Result<String> {
    todo!()
    // match build_contract
    //     .transaction_pattern()
    //     .operation()
    //     .ok_or("ERROR: No specific contract data supplied")?
    // {
    //     Operation::Marketplace {
    //         tokens,
    //         metadata,
    //         selling_price,
    //         ..
    //     } => {
    //         if tokens.is_empty()
    //             || (metadata.is_empty()
    //                 && !(build_contract.action()
    //                     == ContractAction::MarketplaceActions(MarketplaceActions::List)))
    //             || (selling_price <= MIN_ADA * 3
    //                 && (build_contract.action()
    //                     == ContractAction::MarketplaceActions(MarketplaceActions::List)
    //                     || build_contract.action()
    //                         == ContractAction::MarketplaceActions(MarketplaceActions::Update)))
    //         {
    //             return Err(CmdError::Custom {
    //                 str: format!(
    //                     "ERROR wrong data provided for script specific parameters: '{:?}'",
    //                     build_contract.transaction_pattern().operation()
    //                 ),
    //             }
    //             .into());
    //         }
    //     }
    //     _ => {
    //         return Err(CmdError::Custom {
    //             str: format!(
    //                 "ERROR wrong data provided for '{:?}'",
    //                 build_contract.contract_type()
    //             ),
    //         }
    //         .into());
    //     }
    // }

    // let mut gtxd = build_contract.transaction_pattern().into_txdata().await?;
    // let mptxd = build_contract
    //     .transaction_pattern()
    //     .operation()
    //     .unwrap()
    //     .into_mp(gtxd.clone().get_inputs())
    //     .await?;
    // gtxd.set_user_id(build_contract.customer_id as i64);
    // let mut dbsync = drasil_mimir::establish_connection()?;
    // let slot = drasil_mimir::get_slot(&mut dbsync)?;
    // gtxd.set_current_slot(slot as u64);

    // let ret: String;

    // let mpa = if let ContractAction::MarketplaceActions(mpa) = build_contract.action() {
    //     mpa
    // } else {
    //     return Err(Box::new(CmdError::Custom {
    //         str: String::from("Unexpected contract action"),
    //     }));
    // };
    // match mpa {
    //     MarketplaceActions::List => {
    //         use drasil_murin::txbuilder::marketplace::list::*;
    //         //build a listing and send the repsonse to the sender
    //         let contract = TBContracts::get_active_contract_for_user(
    //             build_contract.customer_id as i64,
    //             build_contract.ctype.to_string(),
    //             None,
    //         )?;

    //         let sc_addr = contract.address.to_string();
    //         let sc_version = contract.version.to_string();

    //         let mut dbsync = drasil_mimir::establish_connection()?;
    //         let slot = drasil_mimir::get_slot(&mut dbsync)?;
    //         gtxd.set_current_slot(slot as u64);

    //         let res = build_mp_listing(&gtxd, &mptxd, &sc_addr, &sc_version).await?;

    //         let tx = drasil_murin::utxomngr::RawTx::new(
    //             &res.get_tx_body(),
    //             &res.get_txwitness(),
    //             &res.get_tx_unsigned(),
    //             &res.get_metadata(),
    //             &gtxd.to_string(),
    //             &mptxd.to_string(),
    //             &res.get_used_utxos(),
    //             &hex::encode(gtxd.get_stake_address().to_bytes()),
    //             &(build_contract.customer_id as i64),
    //             &[contract.contract_id],
    //         );

    //         ret = super::create_response(
    //             &res,
    //             &tx,
    //             build_contract.transaction_pattern().wallet_type().as_ref(),
    //         )?
    //         .to_string();
    //     }
    //     MarketplaceActions::Buy => {
    //         ret = "Got MP Buy Transaction".to_string();
    //     }
    //     MarketplaceActions::Cancel => {
    //         ret = "Got MP Cancel Transaction".to_string();
    //     }
    //     MarketplaceActions::Update => {
    //         ret = "Got MP Update Transaction".to_string();
    //     }
    // }
    // Ok(ret)
}
