//! WorldMobile token staking smart contract.

use drasil_murin::utils::to_bignum;
use drasil_murin::worldmobile::configuration::StakingConfig;
use drasil_murin::worldmobile::wmtstaking::stake::{AtStakingBuilder, AtStakingParams};
use drasil_murin::{wallet, TransactionUnspentOutputs, AssetName, PerformTxb};

use super::staking::StakingAction;
use crate::datamodel::ContractAction;
use crate::{BuildContract, CmdError, create_response};

/// Handle WMT staking operations.
pub async fn handle_wmt_staking(build_contract: BuildContract) -> crate::Result<String> {
    // Construct Operations Type from Json Input
    let operation = build_contract
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?;
    
    // Check if contract action is staking
    let ContractAction::StakingAction(StakingAction::Stake) = build_contract.action() else {
        return Err(Box::new(CmdError::Custom {
            str: String::from("Unexpected opreation"),
        }));
    };
    let mut stxd = operation.into_wmt_staking().await?;
    // Create the general transaction data, the data send us from the requesting wallet
    // It should contain at least some addresses we can work with but it can have more. 
    let mut gtxd = build_contract.transaction_pattern().into_txdata().await?;

    // We need to have at least addresses otherwise we can't do anything.
    if gtxd.get_senders_addresses().is_empty() {
        return Err(Box::new(CmdError::Custom {
            str: String::from("No Addresses provided"),
        }));
    }
    let senders_addresses = gtxd.get_senders_addresses();
    // Fetch UTxOs for the given addresses.
    let wallet_utxos =
    senders_addresses
            .iter()
            .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                acc.merge(
                    drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap(),
                );
                acc
            });
    gtxd.set_inputs(wallet_utxos);
    
    // Get the first address which is the unique address to identify a wallet.
    // This is the first address where the staking key was used.
    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&gtxd.get_stake_address().to_hex())?,
    )
    .await?;
    // Set the wallet address to be the first address.
    stxd.set_wallet_addr(&first_address);

    // Set requesting User (This is the API user belonging to the API Token not the user owning the wallet)
    gtxd.set_user_id(build_contract.customer_id as i64);
    
    // Read and set the current Slot in the general transaction data.
    let mut dbsync = drasil_mimir::establish_connection()?;
    let slot = drasil_mimir::get_slot(&mut dbsync)?;
    gtxd.set_current_slot(slot as u64);

    if let None = gtxd.get_collateral() {
        let co_inputs = gtxd.get_inputs().get_coin_only();
        let collateral : TransactionUnspentOutputs = co_inputs.filter(|i| i.output().amount().coin().compare(&to_bignum(10000000)) == -1 ).collect(); 
        if collateral.is_empty() {
            return Err(Box::new(CmdError::Custom {
                str: String::from("No collateral defined and not possible to select random collateral"),
            }));
        }
        gtxd.set_collateral(collateral.get(0));
    }

    // load Staking Constants
    let wmt_staking_config = StakingConfig::load();

    // We need to look for the Registration UTxO holding the ENNFT specified in the sent datum.
    // We know it must be on the Registration Smart Contract and we know the address of this contract. 
    // We also know there must be an ENNFT on the UTxO so we are only interested in UTxO containing Tokens. 
    let registration_utxo = drasil_mimir::get_asset_utxos_on_addr(&mut dbsync, &wmt_staking_config.registration_sc_address)?;
    // We have now all UTxOs containing an asset on the Registration Smart Contract, lets filter the one we want.
    let registration_utxo = registration_utxo.find_utxos_containing_asset(&wmt_staking_config.ennft_policy_id, &AssetName::new( hex::decode(&stxd.ennft)?)?)?.get(0);

    let registration_datum = if let Some(d) = registration_utxo.output().plutus_data() {
        todo!();
        // The Registration Transaction has a decoding function, use it here. 
    } else {
        return Err(Box::new(CmdError::Custom {
            str: String::from("No correct EN registration found"),
        }));
    };
    stxd.set_registration_reference(&registration_utxo);
    stxd.set_registration_datum(&registration_datum);

    log::debug!("Try to build transaction...");
    let txb_param: AtStakingParams = &stxd;

    let minter = AtStakingBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let bld_tx = builder.build(&minter).await?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &stxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(build_contract.customer_id()),
        &[0],
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &bld_tx,
        &tx,
        build_contract.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}
