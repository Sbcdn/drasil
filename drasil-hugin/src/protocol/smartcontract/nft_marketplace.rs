use crate::database::TBContracts;
use crate::datamodel::{ContractAction, MarketplaceActions, Operation};
use crate::{create_response, BuildContract};
use drasil_murin::cardano::get_network_from_address;
use drasil_murin::wallet::reward_address_from_address;
use drasil_murin::{
    wallet, AtMPCancelBuilder, AtMPCancelParam, AtMPListBuilder, AtMPListParam, MurinError,
    PerformTxb, TransactionUnspentOutputs, TxData,
};
pub async fn handle_marketplace(bc: BuildContract) -> crate::Result<String> {
    match bc
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::Marketplace { tokens, .. } => {
            if tokens.is_empty() {
                return Err(format!("ERROR no asset provided",).into());
            }
            if tokens.len() != 1 {
                return Err(format!("ERROR just one asset at a time is supported",).into());
            }
        }
        _ => {
            return Err(format!("ERROR wrong data provided for '{:?}'", bc.contract_type()).into());
        }
    }

    let operation = match bc.txpattern.operation() {
        Some(d) => match d {
            crate::Operation::Marketplace {
                tokens,
                royalties_addr,
                royalties_rate,
                selling_price,
                wallet_addresses,
            } => (
                tokens,
                royalties_addr,
                royalties_rate,
                selling_price,
                wallet_addresses,
            ),
            _ => return Err("Could not deserialze Marketplace Operation".into()),
        },
        None => return Err("Could not deserialze Marketplace Operation".into()),
    };
    let mut gtxd: TxData;
    match bc.transaction_pattern().into_txdata().await {
        Ok(data) => gtxd = data,
        Err(_) => {
            // We only got wallet addresses, determine the txdata on your own
            let wallet_addresses = operation.4.unwrap();
            let wallet_addresses = wallet_addresses.iter().fold(
                Vec::<drasil_murin::clib::address::Address>::new(),
                |mut acc, n| {
                    acc.push(drasil_murin::clib::address::Address::from_bech32(&n).unwrap());
                    acc
                },
            );

            let an_address = &wallet_addresses[0].clone();

            let stake_address = reward_address_from_address(&an_address.clone())?;

            if !wallet_addresses.is_empty() {
                let wallet_utxos =
                    wallet_addresses
                        .iter()
                        .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                            acc.merge(
                                drasil_mimir::get_address_utxos(&n.to_bech32(None).unwrap())
                                    .unwrap(),
                            );
                            acc
                        });
                gtxd = TxData::new(
                    None,
                    wallet_addresses,
                    Some(stake_address),
                    wallet_utxos,
                    get_network_from_address(&an_address.to_bech32(None)?)?,
                    0,
                )?;
            } else {
                return Err("Unable to determine wallet data".into());
            }
        }
    };

    let mut contracts = TBContracts::get_all_contracts_for_user_typed(
        bc.customer_id as i64,
        "nft_marketplace".to_owned(),
    )
    .map_err(|e| e.to_string())?;

    // Only one Marketplace per user allowed if no contract id is specified in the the transaction pattern
    if contracts.is_empty()
        || (contracts.len() > 1 && bc.transaction_pattern().contract_id().is_none())
    {
        return Err("ERROR users marketplace contract is ambigious".into());
    }

    let contract = if bc.transaction_pattern().contract_id().is_none() {
        contracts
            .first()
            .ok_or("ERROR no marketplace contract found")?
            .clone()
    } else {
        contracts.retain(|elem| {
            elem.contract_id == bc.transaction_pattern().contract_id().unwrap() as i64
        });
        contracts[0].clone()
    };

    let smartcontract_inputs =
        drasil_mimir::get_address_utxos(&contract.address).map_err(|e| e.to_string())?;

    // Transform into Marketplace TxData for further processing
    let mptxd = bc
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_mp(smartcontract_inputs)
        .await?;
    gtxd.set_user_id(bc.customer_id as i64);

    let stake_address = gtxd.get_stake_address();
    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&stake_address.to_bech32(None)?)
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?,
    )
    .await?;

    let contract: TBContracts = TBContracts::get_active_contract_for_user(
        bc.customer_id as i64,
        bc.ctype.to_string(),
        None,
    )
    .map_err(|e| e.to_string())?;

    let mut dbsync = drasil_mimir::establish_connection().map_err(|e| e.to_string())?;
    let slot = drasil_mimir::get_slot(&mut dbsync).map_err(|e| e.to_string())?;
    gtxd.set_current_slot(slot as u64);

    let ret: String;

    match bc.action() {
        ContractAction::MarketplaceActions(MarketplaceActions::List) => {
            log::debug!("Try to build transaction...");
            let txb_param: AtMPListParam = (
                &drasil_murin::plutus::PlutusScript::from_bytes(hex::decode(contract.plutus)?)?,
                &drasil_murin::clib::address::Address::from_bech32(&contract.address)?,
                &first_address,
                &mptxd,
            );

            let minter = AtMPListBuilder::new(txb_param);
            let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
            let bld_tx = builder.build(&minter).await?;

            log::debug!("Try to create raw tx...");

            let tx = drasil_murin::utxomngr::RawTx::new(
                &bld_tx.get_tx_body(),
                &bld_tx.get_txwitness(),
                &bld_tx.get_tx_unsigned(),
                &bld_tx.get_metadata(),
                &gtxd.to_string(),
                &mptxd.to_string(),
                &bld_tx.get_used_utxos(),
                &hex::encode(gtxd.get_stake_address().to_bytes()),
                &(bc.customer_id as i64),
                &[contract.contract_id],
            );

            ret = create_response(
                &bld_tx,
                &tx,
                bc.transaction_pattern().wallet_type().as_ref(),
            )?
            .to_string();
        }
        ContractAction::MarketplaceActions(MarketplaceActions::Buy) => {
            ret = "Got MP Buy Transaction".to_string();
        }
        ContractAction::MarketplaceActions(MarketplaceActions::Cancel) => {
            log::debug!("Try to build transaction...");
            let txb_param: AtMPCancelParam = (
                &drasil_murin::plutus::PlutusScript::from_bytes(hex::decode(contract.plutus)?)?,
                &drasil_murin::clib::address::Address::from_bech32(&contract.address)?,
                &first_address,
                &mptxd,
            );

            let minter = AtMPCancelBuilder::new(txb_param);
            let builder = drasil_murin::TxBuilder::new(&gtxd, &vec![]);
            let bld_tx = builder.build(&minter).await?;

            log::debug!("Try to create raw tx...");

            let tx = drasil_murin::utxomngr::RawTx::new(
                &bld_tx.get_tx_body(),
                &bld_tx.get_txwitness(),
                &bld_tx.get_tx_unsigned(),
                &bld_tx.get_metadata(),
                &gtxd.to_string(),
                &mptxd.to_string(),
                &bld_tx.get_used_utxos(),
                &hex::encode(gtxd.get_stake_address().to_bytes()),
                &(bc.customer_id as i64),
                &[contract.contract_id],
            );

            ret = create_response(
                &bld_tx,
                &tx,
                bc.transaction_pattern().wallet_type().as_ref(),
            )?
            .to_string();
        }
        ContractAction::MarketplaceActions(MarketplaceActions::Update) => {
            ret = "Got MP Update Transaction".to_string();
        }
    }
    Ok(ret)
}
