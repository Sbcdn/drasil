use std::cmp::Ordering;

use drasil_gungnir::minting::models::MintProject;
use drasil_gungnir::Whitelist;
use drasil_murin::minter::build_minttx::{AtCMBuilder, AtCMParams};
use drasil_murin::utils::{from_bignum, to_bignum};
use drasil_murin::{wallet, MurinError};
use drasil_murin::{NativeScript, PerformTxb, ServiceFees};

use crate::datamodel::Operation;
use crate::protocol::create_response;
use crate::{discount, BuildMultiSig, TBContracts};
use crate::TBMultiSigLoc;

pub(crate) async fn handle_collection_mint(bms: &BuildMultiSig) -> crate::Result<String> {
    match bms
        .transaction_pattern()
        .operation()
        .ok_or("ERROR: No specific contract data supplied")?
    {
        Operation::NftCollectionMinter { mint_handles } => {
            let err = Err(format!(
                    "ERROR wrong data provided for script specific parameters: '{:?}'", bms.transaction_pattern().operation()).into());
            if mint_handles.is_empty() {
                return err;
            }
            if wallet::address_from_string(&mint_handles[0].addr)
                .await
                .is_err()
            {
                return err;
            } else {
                let payer0 = wallet::address_from_string(&mint_handles[0].addr).await?;
                for mint in &mint_handles {
                    if payer0 != wallet::address_from_string(&mint.addr).await? {
                        return err;
                    }
                }
            }
        }
        _ => {
            return Err(format!("ERROR wrong data provided for '{:?}'", bms.multisig_type()).into());
        }
    }
    log::debug!("Checks okay...");

    log::debug!("Try to create raw data...");
    let minttxd = bms
        .transaction_pattern()
        .operation()
        .unwrap()
        .into_colmintdata()
        .await?;
    let stake_address = minttxd.mint_handles[0].reward_addr()?.to_bech32(None)?;

    let first_address = wallet::address_from_string(
        &drasil_mimir::api::select_addr_of_first_transaction(&stake_address).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?,
    )
    .await?;
    let mut gtxd = bms.transaction_pattern().into_txdata().await?;
    gtxd.set_senders_addresses(vec![first_address.clone()]);
    let mintproject_ids = minttxd
        .mint_handles
        .iter()
        .fold(Vec::<i64>::new(), |mut acc, n| {
            acc.push(n.project_id);
            acc
        });
    gtxd.set_stake_address(minttxd.mint_handles[0].reward_addr()?);
    log::debug!("Check contracts and mint projects...");
    let mut mintprojects = Vec::<(i64, MintProject, TBContracts, Option<TBMultiSigLoc>)>::new();
    let mut contract_ids = Vec::<i64>::new();
    for id in mintproject_ids {
        let p = MintProject::get_mintproject_by_id(id).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let c = TBContracts::get_contract_uid_cid(bms.customer_id(), p.mint_contract_id).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        contract_ids.push(p.mint_contract_id);
        mintprojects.push((id, p, c, None));
    }
    log::debug!("ContractIds: {:?}", contract_ids);
    gtxd.set_contract_id(contract_ids);

    log::debug!("Try to establish database connection...");
    let mut fees = Vec::<ServiceFees>::new();
    let mut ns_scripts = Vec::<NativeScript>::new();
    let mut whitelists = Vec::<Whitelist>::new();

    log::debug!("Try to determine additional data...");
    for c in mintprojects.iter_mut() {
        let kl = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &c.2.contract_id,
            &c.2.user_id,
            &c.2.version,
        ).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        c.3 = Some(kl.clone());
        if let Some(addr) = kl.fee_wallet_addr {
            fees.push(ServiceFees {
                fee: to_bignum(kl.fee.unwrap() as u64),
                fee_addr: wallet::address_from_string(&addr).await?,
            });
        }
        ns_scripts.push(NativeScript::from_bytes(hex::decode(&c.2.plutus)?)?);
        if let Some(whitelist) = &c.1.whitelists {
            for w in whitelist.iter() {
                whitelists.push(drasil_gungnir::Whitelist::get_whitelist(&c.2.user_id, w).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?);
            }
        }
    }

    // Fees and Discount
    fees.sort_by(|a, b| match a.fee.compare(&b.fee) {
        0 => Ordering::Equal,
        1 => Ordering::Greater,
        _ => Ordering::Less,
    });
    // Contains highest contract fee
    let mut fees = match fees.last() {
        Some(s) => vec![s.clone()],
        None => vec![],
    };

    let mut r = Vec::<i64>::new();
    for c in &mintprojects {
        r.push(discount(gtxd.get_inputs(), c.2.contract_id, c.1.user_id));
    }
    r.sort();
    let discnt = r[r.len() - 1];
    if discnt > 0 {
        let fee = from_bignum(&fees[0].fee);
        fees[0].fee = to_bignum(fee - (fee as f64 * (discnt as f64 / 100.0)) as u64);
        if fee == 0 {
            fees = vec![];
        }
    }
    log::debug!("Check nft data...");
    for m in &minttxd.mint_handles {
        let mintreward = drasil_gungnir::minting::models::MintReward::get_mintreward_by_id(m.id);
        match mintreward {
            Ok(mr) => {
                if mr.processed
                    || mr.minted
                    || mr.project_id != m.project_id
                    || mr.pay_addr != first_address.to_bech32(None).unwrap()
                {
                    return Err(format!("ERROR invalid mint reward '{mr:?}'").into());
                }
                let mut tv = drasil_murin::clib::utils::Value::zero();
                for nft in &mr.v_nfts_b {
                    let v = drasil_murin::clib::utils::Value::from_bytes(nft.to_owned())?;
                    tv = tv.checked_add(&v)?;
                }
                if let Some(x) = m.value()?.compare(&tv) {
                    if x != 0 {
                        return Err(format!("ERROR claim values dont match '{mr:?}'").into());
                    }
                }
            }
            Err(e) => {
                return Err(format!("ERROR mint reward does not exist: '{e:?}'").into());
            }
        }
    }

    // create transaction specific metadata
    let mut metadataassets = Vec::<drasil_murin::minter::AssetMetadata>::new();
    let mut nfts = Vec::<drasil_gungnir::minting::models::Nft>::new();

    for mh in &minttxd.mint_handles {
        let nft_ids = mh.nft_ids()?;
        for nftb in nft_ids {
            let mp: Vec<_> = mintprojects
                .iter()
                .filter(|n| n.0 == mh.project_id)
                .collect();
            let nft = drasil_gungnir::minting::models::Nft::get_nft_by_assetnameb(
                mh.project_id,
                &mp[0].1.nft_table_name,
                &nftb.name(),
            ).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
            if nft.minted {
                return Err(format!("ERROR Nft already minted: '{mh:?}'").into());
            }
            if let Some(metadata) = &nft.metadata {
                metadataassets.push(serde_json::from_str(metadata)?)
            }
            nfts.push(nft);
        }
    }

    let metadata = drasil_murin::minter::Cip25Metadata {
        assets: metadataassets,
        other: None,
        version: "1.0".to_string(),
    };

    log::debug!("Try to determine slot...");
    let mut dbsync = match drasil_mimir::establish_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()).into());
        }
    };
    let slot = match drasil_mimir::get_slot(&mut dbsync) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                    "ERROR could not determine current slot: '{:?}'",
                    e.to_string()
                ).into());
        }
    };
    gtxd.set_current_slot(slot as u64);

    let mut pvks = Vec::<String>::new();
    let mut scripts = Vec::<NativeScript>::new();
    let mut contract_ids = Vec::<i64>::new();
    for m in mintprojects {
        let ident =
            crate::encryption::mident(&m.2.user_id, &m.2.contract_id, &m.2.version, &m.2.address);
        let pvs = crate::encryption::decrypt_pkvs(m.3.unwrap().pvks, &ident).await?;
        pvks.extend(pvs.iter().map(|n| n.to_owned()));
        scripts.push(NativeScript::from_bytes(hex::decode(m.2.plutus)?)?);
        contract_ids.push(m.2.contract_id);
    }

    // TODO: Prices are not implemented yet

    log::debug!("Try to build transaction...");
    let txb_param: AtCMParams = (&scripts, &None, &metadata, &Some(fees), &minttxd);

    let minter = AtCMBuilder::new(txb_param);
    let builder = drasil_murin::TxBuilder::new(&gtxd, &pvks);
    let bld_tx = builder.build(&minter).await?;

    log::debug!("Try to create raw tx...");
    let tx = drasil_murin::utxomngr::RawTx::new(
        &bld_tx.get_tx_body(),
        &bld_tx.get_txwitness(),
        &bld_tx.get_tx_unsigned(),
        &bld_tx.get_metadata(),
        &gtxd.to_string(),
        &minttxd.to_string(),
        &bld_tx.get_used_utxos(),
        &hex::encode(gtxd.get_stake_address().to_bytes()),
        &(bms.customer_id()),
        &contract_ids,
    );
    debug!("RAWTX data: {:?}", tx);

    log::debug!("Try to create response data...");
    let ret = create_response(
        &bld_tx,
        &tx,
        bms.transaction_pattern().wallet_type().as_ref(),
    )?;
    Ok(serde_json::json!(ret).to_string())
}
