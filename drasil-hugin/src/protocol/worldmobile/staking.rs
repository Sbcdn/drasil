//! Staking data model.

use std::str::FromStr;

use drasil_mimir::{self, Datum};
use drasil_murin::utils::to_bignum;
use drasil_murin::worldmobile::configuration::StakingConfig;
use drasil_murin::worldmobile::enreg::restore_wmreg_datum;
use drasil_murin::worldmobile::wmtstaking::stake::AtStakingBuilder;
use drasil_murin::worldmobile::wmtstaking::unstake::AtUnStakingBuilder;
use drasil_murin::worldmobile::wmtstaking::StakeDatum;
use drasil_murin::{
    wallet, AssetName, MurinError, PerformTxb, TransactionUnspentOutput, TransactionUnspentOutputs,
};
use serde::{Deserialize, Serialize};
use strum_macros::Display;

use crate::datamodel::ContractAction;
use crate::error::SystemDBError as Error;
use crate::{create_response, BuildContract};

/// The `Action` type enumerates all the smart contract actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum StakingAction {
    /// Staking a WMT
    Stake,
    /// Unstake a WMT
    UnStake,
}

impl FromStr for StakingAction {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase();
        let action = match value.as_str() {
            "stake" => Self::Stake,
            "unstake" => Self::UnStake,
            _ => return Err(Error::InvalidContractAction(value)),
        };
        Ok(action)
    }
}

/// Handle WMT staking operations.
pub async fn handle_wmt_stake(build_contract: BuildContract) -> crate::Result<String> {
    // Construct Operations Type from Json Input
    let operation = build_contract
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?;

    // Check if contract action is staking
    let ContractAction::StakingAction(StakingAction::Stake) = build_contract.action() else {
        return Err(String::from("Unexpected opreation").into());
    };
    let mut stxd = operation.into_wmt_staking().await?;
    // Create the general transaction data, the data send us from the requesting wallet
    // It should contain at least some addresses we can work with but it can have more.
    let mut gtxd = build_contract.transaction_pattern().into_txdata().await?;

    // We need to have at least addresses otherwise we can't do anything.
    if gtxd.get_senders_addresses().is_empty() {
        return Err(String::from("No Addresses provided").into());
    }
    let senders_addresses = gtxd.get_senders_addresses();
    // Fetch the wallet UTxOs for the given addresses.
    let wallet_utxos =
        senders_addresses
            .iter()
            .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                acc.merge(drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap());
                acc
            });
    gtxd.set_inputs(wallet_utxos);

    // Get the first address which is the unique address identifying a wallet.
    // This is the first address that used the staking key.
    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&gtxd.get_stake_address().to_hex())
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?,
    )
    .await?;
    // Set the wallet address to be the first address.
    stxd.wallet_addr = Some(first_address);

    // Set request User associated with the API Token.
    // This user is different from the user owning the wallet
    gtxd.set_user_id(build_contract.customer_id as i64);

    // Read and set the current Slot in the general transaction data.
    let mut dbsync = drasil_mimir::establish_connection()
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    let slot = drasil_mimir::get_slot(&mut dbsync)
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    gtxd.set_current_slot(slot as u64);

    // Set the transaction collateral data if it doesn't exist.
    if gtxd.get_collateral().is_none() {
        let co_inputs = gtxd.get_inputs().get_coin_only();
        let collateral: TransactionUnspentOutputs = co_inputs
            .filter(|i| i.output().amount().coin().compare(&to_bignum(10000000)) == -1)
            .collect();
        if collateral.is_empty() {
            return Err(String::from(
                "No collateral defined and not possible to select random collateral",
            )
            .into());
        }
        gtxd.set_collateral(collateral.get(0));
    }

    // load Staking configuration
    let wmt_staking_config = StakingConfig::load();

    // We need to look for the Registration UTxO holding the ENNFT specified in the sent datum.
    // We know it must be on the Registration Smart Contract and we know the address of this contract.
    // We also know there must be an ENNFT on the UTxO so we are only interested in UTxO containing Tokens.
    let registration_utxo = drasil_mimir::get_asset_utxos_on_addr(
        &mut dbsync,
        &wmt_staking_config.registration_sc_address,
    )
    .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    // Now we have all UTxOs containing an asset on the Registration Smart Contract, lets filter the one we want.
    let registration_utxo = registration_utxo
        .find_utxos_containing_asset(
            &wmt_staking_config.ennft_policy_id,
            &AssetName::new(hex::decode(&stxd.ennft)?)?,
        )?
        .get(0);

    let registration_datum = if let Some(d) = registration_utxo.output().plutus_data() {
        restore_wmreg_datum(&d.to_bytes())?
    } else {
        return Err(String::from("No correct EN registration found").into());
    };
    stxd.registration_reference = Some(registration_utxo);
    stxd.registration_datum = Some(registration_datum);

    log::debug!("Try to build transaction...");

    let minter = AtStakingBuilder::new(&stxd);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let tx_builder_out = builder.build(&minter).await?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &tx_builder_out.get_tx_body(),
        &tx_builder_out.get_txwitness(),
        &tx_builder_out.get_tx_unsigned(),
        &tx_builder_out.get_metadata(),
        &gtxd.to_string(),
        &stxd.to_string(),
        &tx_builder_out.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(build_contract.customer_id()),
        &[0],
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &tx_builder_out,
        &tx,
        build_contract.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}

/// Handle WMT un-staking operation.
pub async fn handle_wmt_unstake(build_contract: BuildContract) -> crate::Result<String> {
    // Construct Operations Type from Json Input
    let operation = build_contract
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?;

    // Check if contract action is staking
    let ContractAction::StakingAction(StakingAction::UnStake) = build_contract.action() else {
        return Err(String::from("Unexpected opreation").into());
    };

    let mut unstake_data = operation.into_wmt_unstaking().await?;
    let mut dbsync = drasil_mimir::establish_connection()
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    // load Staking configuration
    let wmt_staking_config = StakingConfig::load();

    // Get the utxos-datum for this address.
    let data = drasil_mimir::get_smart_contract_utxos(
        &wmt_staking_config.registration_sc_address,
        &mut dbsync,
    )
    .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    let utxos = filter_utxo_by_ennft(unstake_data.ennft.as_bytes(), data);

    unstake_data.smart_contract_utxos.replace(utxos);
    // Create the general transaction data, the data send us from the requesting wallet
    // It should contain at least some addresses we can work with but it can have more.
    let mut gtxd = build_contract.transaction_pattern().into_txdata().await?;

    // We need to have at least addresses otherwise we can't do anything.
    if gtxd.get_senders_addresses().is_empty() {
        return Err(String::from("No Addresses provided").into());
    }
    let senders_addresses = gtxd.get_senders_addresses();
    // Fetch the wallet UTxOs for the given addresses.
    let wallet_utxos =
        senders_addresses
            .iter()
            .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                acc.merge(drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap()).unwrap());
                acc
            });
    gtxd.set_inputs(wallet_utxos);

    // Get the first address which is the unique address identifying a wallet.
    // This is the first address that used the staking key.
    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&gtxd.get_stake_address().to_hex())
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?,
    )
    .await?;
    // Set the wallet address to be the first address.
    unstake_data.wallet_addr = Some(first_address);

    // Set request User associated with the API Token.
    // This user is different from the user owning the wallet
    gtxd.set_user_id(build_contract.customer_id as i64);

    // Read and set the current Slot in the general transaction data.
    let mut dbsync = drasil_mimir::establish_connection()
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    let slot = drasil_mimir::get_slot(&mut dbsync)
        .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    gtxd.set_current_slot(slot as u64);

    // Set the transaction collateral data if it doesn't exist.
    if gtxd.get_collateral().is_none() {
        let co_inputs = gtxd.get_inputs().get_coin_only();
        let collateral: TransactionUnspentOutputs = co_inputs
            .filter(|i| i.output().amount().coin().compare(&to_bignum(10000000)) == -1)
            .collect();
        if collateral.is_empty() {
            return Err(String::from(
                "No collateral defined and not possible to select random collateral",
            )
            .into());
        }
        gtxd.set_collateral(collateral.get(0));
    }

    // load Staking configuration
    let wmt_staking_config = StakingConfig::load();

    // We need to look for the Registration UTxO holding the ENNFT specified in the sent datum.
    // We know it must be on the Registration Smart Contract and we know the address of this contract.
    // We also know there must be an ENNFT on the UTxO so we are only interested in UTxO containing Tokens.
    let registration_utxo = drasil_mimir::get_asset_utxos_on_addr(
        &mut dbsync,
        &wmt_staking_config.registration_sc_address,
    )
    .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
    // Now we have all UTxOs containing an asset on the Registration Smart Contract, lets filter the one we want.
    let registration_utxo = registration_utxo
        .find_utxos_containing_asset(
            &wmt_staking_config.ennft_policy_id,
            &AssetName::new(hex::decode(&unstake_data.ennft)?)?,
        )?
        .get(0);

    unstake_data.registration_reference = Some(registration_utxo);

    log::debug!("Try to build transaction...");

    let minter = AtUnStakingBuilder::new(&unstake_data);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
    let tx_builder_out = builder.build(&minter).await?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &tx_builder_out.get_tx_body(),
        &tx_builder_out.get_txwitness(),
        &tx_builder_out.get_tx_unsigned(),
        &tx_builder_out.get_metadata(),
        &gtxd.to_string(),
        &unstake_data.to_string(),
        &tx_builder_out.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(build_contract.customer_id()),
        &[0],
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &tx_builder_out,
        &tx,
        build_contract.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}

// The utxo will sit on the staking smart contract.
// We know the address (address of the staking sc) of what we have to look for.
// We also know that we have staked to the EN with the token we got.
// We query all the utxo in the staking smart contract
// Filter all the EN with NFT we are interested in.
// Filter based on the user id
//

/// Returns all the UTxO with the given earth node NFT.
pub(crate) fn filter_utxo_by_ennft(
    ennft: &[u8],
    data: Vec<(TransactionUnspentOutput, Datum)>,
) -> TransactionUnspentOutputs {
    let (utxos, _): (Vec<_>, Vec<Datum>) = data
        .into_iter()
        .filter(|(_, datum)| {
            let result = serde_json::from_slice::<StakeDatum>(&datum.bytes);
            matches!(result, Ok(sdata) if sdata.ennft == ennft)
        })
        .unzip();

    TransactionUnspentOutputs::from_iter(utxos.iter())
}
