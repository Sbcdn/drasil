use drasil_gungnir::minting::models::{MintProject, MintReward, Nft};
use drasil_gungnir::{SpecificAsset, Whitelist, WhitelistType, WlAlloc, WlEntry};
use drasil_hugin::TBContracts;
use drasil_murin::{clib::Assets, utils::to_bignum, wallet, AssetName, MultiAsset, PolicyID};
use serde::{Deserialize, Serialize};

use crate::SleipnirError;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WlNew {
    pub max_addr_repeat: i32,
    pub wl_type: WhitelistType,
    pub description: String,
    pub notes: String,
}

// create Whitelist on Database
pub fn create_whitelist(user_id: &i64, data: &Vec<WlNew>) -> Result<Vec<Whitelist>, SleipnirError> {
    log::debug!("Sleipnir create whitelists: {:?}", data);
    let mut result = Vec::<Whitelist>::new();
    for n in data {
        let r = Whitelist::create_whitelist(
            user_id,
            &n.max_addr_repeat,
            &n.wl_type,
            &n.description,
            &n.notes,
        );
        log::debug!("Whitelist add result: {:?}", r);
        result.push(r?)
    }

    Ok(result)
}

// Delete Whitelist
pub fn delete_whitelists(user_id: &i64, wl_id: &i64) -> Result<usize, SleipnirError> {
    Ok(Whitelist::remove_wl(user_id, wl_id)?)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WhitelistEntry {
    pub payment_address: String,
    pub stake_address: Option<String>,
    pub wl: i64,
    pub specific_asset: Option<SpecificAsset>,
}

fn equal_wl(entry: &[WhitelistEntry]) -> bool {
    let wl = entry[0].wl;
    entry.iter().fold(true, |mut acc, n| {
        if n.wl != wl {
            acc = false
        }
        acc
    })
}

pub fn add_whitelistentry(
    user_id: &i64,
    entry: &Vec<WhitelistEntry>,
) -> Result<Vec<WlAlloc>, SleipnirError> {
    let mut result = Vec::<WlAlloc>::new();
    if equal_wl(entry) && !entry.is_empty() {
        Whitelist::get_whitelist(user_id, &entry[0].wl)?;
    } else {
        return Err(SleipnirError::new(
            "Cannot add entry to whitelist, preconditions not met",
        ));
    }

    for e in entry {
        result.push(Whitelist::add_to_wl(
            &e.wl,
            &e.payment_address,
            e.stake_address.as_ref(),
            e.specific_asset.as_ref(),
        )?);
    }

    Ok(result)
}

pub fn delete_whitelistentry(user_id: &i64, entry: &[WhitelistEntry]) -> Result<(), SleipnirError> {
    for e in entry.iter() {
        Whitelist::remove_from_wl(user_id, &e.wl, &e.payment_address)?;
    }
    Ok(())
}

pub fn allocate_asset_to_whitelistentry(
    user_id: &i64,
    wl_id: &i64,
    payment_address: &String,
    asset: &[SpecificAsset],
) -> Result<(), SleipnirError> {
    Whitelist::get_whitelist(user_id, wl_id)?;
    let wl_entry = WlAlloc::get_address_whitelist(wl_id, payment_address)?;
    for (i, s) in asset.iter().enumerate() {
        if wl_entry.len() >= i {
            WlAlloc::update_alloc(wl_id, &wl_entry[i].alloc_id, Some(s))?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn deallocate_asset_from_whitelistentry(
    user_id: &i64,
    wl_id: &i64,
    payment_address: &String,
    asset: &Vec<SpecificAsset>,
) -> Result<(), SleipnirError> {
    Whitelist::get_whitelist(user_id, wl_id)?;
    let sa = serde_json::to_value(asset)?;
    let wl_entry = WlAlloc::get_specific_address_allocations(payment_address, &sa)?;
    WlAlloc::update_alloc(&wl_entry.wl, &wl_entry.alloc_id, None)?;

    Ok(())
}

pub async fn random_allocation_whitelist_to_mintproject(
    user_id_in: &i64,
    project_id_in: &i64,
    whitelist_id_in: &i64,
) -> Result<(), SleipnirError> {
    let wl = Whitelist::get_whitelist(user_id_in, whitelist_id_in)?;
    let mp = MintProject::get_mintproject_by_id(*project_id_in)?;
    if wl.user_id != mp.user_id {
        return Err(SleipnirError::new("Error: Precondition not met"));
    }
    let wl_entry = WlAlloc::get_whitelist_entries(user_id_in, whitelist_id_in)?;
    let mint_contract =
        drasil_hugin::database::TBContracts::get_contract_uid_cid(mp.user_id, mp.mint_contract_id)?;

    'a: for entry in wl_entry {
        let payaddr = match drasil_mimir::select_addr_of_first_transaction(
            match &wallet::bech32_stake_address_from_str(&entry.payment_address) {
                Ok(o) => o,
                Err(_) => {
                    log::debug!("Could not determine a first address for stake addr, original listed address was: {:?}", &entry.payment_address);
                    continue 'a;
                }
            },
        ) {
            Ok(o) => o,
            Err(_) => {
                log::debug!("Could not determine a first address for stake addr, original listed address was: {:?}", &entry.payment_address);
                continue 'a;
            }
        };

        let existing = drasil_gungnir::minting::models::MintReward::get_mintrewards_by_pid_addr(
            *project_id_in,
            &payaddr,
        )?;

        if let Some(max) = mp.max_mint_p_addr {
            // Claim an NFT
            if existing.len() < max.try_into().unwrap() {
                let claim = drasil_gungnir::minting::models::Nft::claim_random_unminted_nft(
                    mp.id,
                    &mp.nft_table_name,
                    &payaddr,
                    0,
                )
                .await?;

                if let Some(nft) = claim {
                    // Create Mint Reward

                    let mut mint_value = drasil_murin::clib::utils::Value::zero();
                    let mut assets = Assets::new();
                    assets.insert(&AssetName::new(nft.asset_name_b.clone())?, &to_bignum(1));
                    let mut ma = MultiAsset::new();
                    ma.insert(
                        &PolicyID::from_hex(mint_contract.policy_id.as_ref().unwrap()).unwrap(),
                        &assets,
                    );
                    mint_value.set_multiasset(&ma);

                    drasil_gungnir::minting::models::MintReward::create_mintreward(
                        *user_id_in,
                        mp.mint_contract_id,
                        &payaddr,
                        vec![&nft.asset_name_b],
                        vec![&mint_value.to_bytes()],
                    )?;
                }
            }
        }
    }

    Ok(())
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AllocateSpecificAssetsToMintProject {
    pub project_id_in: i64,
    pub whitelist_id_in: i64,
}

pub async fn allocate_specific_assets_to_mintproject(
    user_id_in: &i64,
    project_id_in: &i64,
    whitelist_id_in: &i64,
) -> Result<(), SleipnirError> {
    let wl = Whitelist::get_whitelist(user_id_in, whitelist_id_in)?;
    let mp = MintProject::get_mintproject_by_id(*project_id_in)?;
    if wl.user_id != mp.user_id {
        return Err(SleipnirError::new("Error: Precondition not met"));
    }
    let wl_entry = WlAlloc::get_whitelist_entries(user_id_in, whitelist_id_in)?;

    'a: for entry in wl_entry {
        if let Some(v) = entry.specific_asset {
            log::debug!("Found specific asset");
            let anb = serde_json::from_value::<SpecificAsset>(v).unwrap();
            log::debug!("{:?}", anb);

            if anb.project_id == mp.id {
                let payaddr = match drasil_mimir::select_addr_of_first_transaction(
                    match &wallet::bech32_stake_address_from_str(&entry.payment_address) {
                        Ok(o) => o,
                        Err(_) => {
                            log::debug!("Could not determine a first address for stake addr, original listed address was: {:?}", &entry.payment_address);
                            continue 'a;
                        }
                    },
                ) {
                    Ok(o) => o,
                    Err(_) => {
                        log::debug!("Could not determine a first address for stake addr, original listed address was: {:?}", &entry.payment_address);
                        continue 'a;
                    }
                };
                let binassetname = &match hex::decode(anb.assetname_b.clone()) {
                    Ok(o) => o,
                    Err(_) => continue 'a,
                };
                log::debug!("Got binassetname: {:?}", binassetname);
                let k = Nft::set_nft_claim_addr(&mp.id, &mp.nft_table_name, binassetname, &payaddr)
                    .await;
                log::debug!("Claimed NFT: {:?}", k);
                k?;

                let nft = Nft::get_nft_by_assetnameb(mp.id, &mp.nft_table_name, binassetname)?;
                log::debug!("NFT: {:?}\n Try to create claim...", nft);
                if let Some(claim_addr) = nft.claim_addr {
                    if payaddr == claim_addr && !nft.minted {
                        let mint_contract =
                            TBContracts::get_contract_uid_cid(*user_id_in, mp.mint_contract_id)?;
                        let mut mint_value = drasil_murin::clib::utils::Value::zero();
                        let mut assets = Assets::new();
                        assets.insert(
                            &AssetName::new(binassetname.clone())?,
                            &to_bignum(anb.amount),
                        );
                        let mut ma = MultiAsset::new();
                        ma.insert(
                            &PolicyID::from_bytes(hex::decode(mint_contract.policy_id.unwrap())?)
                                .unwrap(),
                            &assets,
                        );
                        mint_value.set_multiasset(&ma);

                        if MintReward::create_mintreward(
                            *user_id_in,
                            mp.mint_contract_id,
                            &payaddr,
                            vec![binassetname],
                            vec![&mint_value.to_bytes()],
                        )
                        .is_err()
                        {
                            if let Ok(mrwd) =
                                MintReward::get_mintreward_by_nft_ids(vec![binassetname.to_vec()])
                            {
                                if mrwd.project_id == anb.project_id
                                    && mrwd.v_nfts_b == vec![mint_value.to_bytes()]
                                    && mrwd.pay_addr != payaddr
                                {
                                    MintReward::update_payaddr(mrwd.id, &payaddr)?;
                                } else if mrwd.project_id == anb.project_id
                                    && mrwd.v_nfts_b == vec![mint_value.to_bytes()]
                                    && mrwd.pay_addr == payaddr
                                {
                                    log::debug!("all up to date nothing to do ...")
                                } else {
                                    log::error!(
                                        "Something is wrong with this mint reward: {}",
                                        mrwd.id
                                    )
                                }
                            } else {
                                continue 'a;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ImportWhitelistFromCSV {
    pub whitelist_id: i64,
    pub project_id: Option<i64>,
    pub csv: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct CSVWhitelist {
    pub addr: String,
    pub stake_address: Option<String>,
    pub assetname_b: Option<String>,
    pub fingerprint: Option<String>,
    pub amount: Option<u64>,
}

pub fn import_whitelist_from_csv(
    user_id: &i64,
    whitelist_id: &i64,
    project_id: Option<&i64>,
    csv: &[u8],
) -> Result<i64, SleipnirError> {
    let wl = Whitelist::get_whitelist(user_id, whitelist_id)?;
    let mut rdr = csv::Reader::from_reader(csv);
    let mut trdr = csv::Reader::from_reader(csv);
    let mut binding = csv::Reader::from_reader(csv);
    let headers = if let Ok(o) = binding.headers() {
        Some(o)
    } else {
        None
    };
    let n = trdr.records().count();
    log::debug!("Count: {:?}", n);
    log::debug!("Reader has headers: {:?}", headers);
    let mut counter = 0;

    for result in rdr.records() {
        log::debug!("A Record was found: {:?}", result);
        if let Ok(record) = result {
            match record.deserialize::<CSVWhitelist>(headers) {
                Ok(csvwe) => {
                    log::debug!("Record deserialized: {:?}", csvwe);
                    let stake = if csvwe.stake_address.is_none() {
                        Some(wallet::bech32_stake_address_from_str(&csvwe.addr)?)
                    } else {
                        csvwe.stake_address
                    };

                    let sa = if let Some(id) = project_id {
                        if csvwe.assetname_b.is_none()
                            || csvwe.fingerprint.is_none()
                            || csvwe.amount.is_none()
                        {
                            None
                        } else {
                            Some(SpecificAsset {
                                project_id: *id,
                                assetname_b: csvwe.assetname_b.unwrap(),
                                fingerprint: csvwe.fingerprint.unwrap(),
                                amount: csvwe.amount.unwrap(),
                            })
                        }
                    } else {
                        None
                    };

                    match Whitelist::add_to_wl(&wl.id, &csvwe.addr, stake.as_ref(), sa.as_ref()) {
                        Ok(_) => {
                            counter += 1;
                        }
                        Err(e) => {
                            log::error!("Error: could not add to whitelist: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error during whitelist import: {:?}", e);
                }
            }
        } else {
            log::debug!("No more records; processed: {}", counter);
        }
    }
    Ok(counter)
}

pub fn get_whitelist_entrys(
    user_id: &i64,
    whitelist_id: &i64,
) -> Result<Vec<WlEntry>, SleipnirError> {
    let result = WlAlloc::get_whitelist_entries(user_id, whitelist_id)?;
    Ok(result)
}

pub fn get_asset_unalloc_whitelist_entrys() -> Result<Vec<WlEntry>, SleipnirError> {
    Ok(vec![])
}

pub fn get_asset_alloc_whitelist_entrys() -> Result<Vec<WlEntry>, SleipnirError> {
    Ok(vec![])
}
