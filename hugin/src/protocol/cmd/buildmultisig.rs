/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::{MultiSigType, OneShotReturn, ScriptSpecParams, TransactionPattern};
use crate::{CmdError, Parse, TBContracts};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
//use gungnir::schema::whitelist;
use mimir::MurinError;
use serde_json::json;
use tracing::{debug, instrument};

use rand::Rng;

#[derive(Debug, Clone)]
pub struct BuildMultiSig {
    customer_id: u64,
    mtype: MultiSigType,
    txpattern: TransactionPattern,
}

impl BuildMultiSig {
    pub fn new(cid: u64, mtype: MultiSigType, txpatter: TransactionPattern) -> BuildMultiSig {
        BuildMultiSig {
            customer_id: cid,
            mtype,
            txpattern: txpatter,
        }
    }

    pub fn customer_id(&self) -> i64 {
        self.customer_id as i64
    }

    pub fn multisig_type(&self) -> MultiSigType {
        self.mtype.clone()
    }

    pub fn transaction_pattern(&self) -> TransactionPattern {
        self.txpattern.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<BuildMultiSig> {
        let customer_id = parse.next_int()?;
        let mtype = parse.next_bytes()?;
        let mtype: MultiSigType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&mtype)?;
        let txpattern = parse.next_bytes()?;
        let txpattern: TransactionPattern = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txpattern)?;
        Ok(BuildMultiSig {
            customer_id,
            mtype,
            txpattern,
        })
    }

    #[instrument(skip(self, dst))]
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        dotenv::dotenv().ok();
        let mut response =
            Frame::Simple("ERROR: Could not build multisignature transaction".to_string());
        if self.multisig_type() != MultiSigType::ClAPIOneShotMint {
            if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
                debug!(?response);
                response = Frame::Simple(e.to_string());
                dst.write_frame(&response).await?;
            }
            log::debug!("Transaction pattern check okay!");
        }

        let mut ret = String::new();
        match self.multisig_type() {
            MultiSigType::SpoRewardClaim => {
                ret = match self.handle_rewardclaim().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::NftVendor => {}
            MultiSigType::Mint => {
                ret = match self.handle_collection_mint().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::ClAPIOneShotMint => {
                ret = match self.handle_onehshot_mint().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::TestRewards => {
                ret = match self.handle_testrewards().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::CustomerPayout => {
                ret = match self.handle_customer_payout().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            _ => {}
        }

        response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret)?,
        ));
        debug!(?response);
        dst.write_frame(&response).await?;

        Ok(())
    }

    async fn handle_rewardclaim(&self) -> crate::Result<String> {
        info!("verify transaction data...");
        match self
            .transaction_pattern()
            .script()
            .ok_or("ERROR: No specific contract data supplied")?
        {
            ScriptSpecParams::SpoRewardClaim {
                reward_tokens,
                recipient_stake_addr,
                recipient_payment_addr,
            } => {
                let err = Err(CmdError::Custom {
                    str: format!(
                        "ERROR wrong data provided for script specific parameters: '{:?}'",
                        self.transaction_pattern().script()
                    ),
                }
                .into());
                if reward_tokens.is_empty()
                    || murin::b_decode_addr(&recipient_stake_addr).await.is_err()
                    || murin::b_decode_addr(&recipient_payment_addr).await.is_err()
                    || murin::wallet::get_stake_address(
                        &murin::b_decode_addr(&recipient_stake_addr).await?,
                    )? != murin::wallet::get_stake_address(
                        &murin::b_decode_addr(&mimir::api::select_addr_of_first_transaction(
                            &murin::decode_addr(&recipient_stake_addr)
                                .await?
                                .to_bech32(None)
                                .unwrap(),
                        )?)
                        .await?,
                    )?
                {
                    return err;
                }
            }
            _ => {
                return Err(CmdError::Custom {
                    str: format!("ERROR wrong data provided for '{:?}'", self.multisig_type()),
                }
                .into());
            }
        }

        info!("create raw data...");
        let mut rwdtxd = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_rwd()
            .await?;
        rwdtxd.set_payment_addr(
            &murin::b_decode_addr(&mimir::api::select_addr_of_first_transaction(
                &rwdtxd
                    .get_stake_addr()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for stake address"),
            )?)
            .await?,
        );
        let mut gtxd = self.transaction_pattern().into_txdata().await?;
        gtxd.set_user_id(self.customer_id);

        info!("establish database connections...");
        let drasildbcon = crate::database::drasildb::establish_connection()?;
        let gcon = gungnir::establish_connection()?;

        info!("determine contract...");
        let mut contract = determine_contract(gtxd.get_contract_id(), self.customer_id as i64)?;
        if contract.is_none() {
            info!("No contract ID provided, try to select contract automatically");
            let contracts = crate::drasildb::TBContracts::get_all_contracts_for_user_typed(
                self.customer_id as i64,
                crate::MultiSigType::SpoRewardClaim.to_string(),
            )?;
            let mut all_tokens_available = Vec::<bool>::new();
            for c in contracts {
                all_tokens_available.extend(rwdtxd.get_reward_tokens().iter().map(|t| {
                    let fingerprint = murin::chelper::make_fingerprint(
                        &hex::encode(t.0.to_bytes()),
                        &hex::encode(t.1.name()),
                    )
                    .unwrap();
                    let has_wl = gungnir::TokenWhitelist::has_contract_valid_whitelisting(
                        c.contract_id,
                        self.customer_id as i64,
                        &fingerprint,
                    )
                    .unwrap();
                    if has_wl {
                        // Check if whitelist token on contract also has available rewards
                        gungnir::Rewards::get_available_rewards(
                            &gcon,
                            &gtxd.get_stake_address().to_bech32(None).expect(
                                "ERROR Could not construct bech32 address for stake address",
                            ),
                            &rwdtxd.get_payment_addr().to_bech32(None).expect(
                                "ERROR Could not construct bech32 address for payment address",
                            ),
                            &fingerprint,
                            c.contract_id as i64,
                            self.customer_id() as i64,
                            murin::clib::utils::from_bignum(&t.2) as i64,
                        )
                        .is_ok()
                    } else {
                        false
                    }
                }));
                let sum = all_tokens_available.iter().fold(true, |n, s| *s && n);
                debug!("Available Tokens: {:?}", all_tokens_available);
                debug!("Sum: {:?}", sum);

                if sum {
                    contract = Some(c.clone());
                    break;
                } else {
                    all_tokens_available = Vec::<bool>::new();
                }
            }

            if contract.is_none() {
                return Err(CmdError::Custom {
                    str: "Automatic selection failed, no suitable contract found.".to_string(),
                }
                .into());
            }
        }
        let contract = contract.expect("Error: Could not unwrap contract");
        info!(
            "Contract selected: UID: '{}', CID '{}'",
            self.customer_id, contract.contract_id
        );

        info!("check vesting periods...");
        // Filter Tokens which are in a vesting period out of the reward tokens
        // get the token whitelist of the contract
        let vesting_whitelist = gungnir::TokenWhitelist::get_in_vesting_filtered_whitelist(
            contract.contract_id,
            self.customer_id as i64,
        )?;
        // for the remaining whitelisting filter the tokens out of reward tokens.
        let mut rwd_tokens = rwdtxd.get_reward_tokens();
        for vt in vesting_whitelist {
            let policy = murin::chelper::string_to_policy(&vt.policy_id)?;
            let assetname = match vt.tokenname {
                Some(tn) => murin::chelper::string_to_assetname(&tn)?,
                None => murin::clib::AssetName::new(b"".to_vec())
                    .expect("Could not create emtpy tokenname"),
            };
            rwd_tokens.retain(|n| n.0 != policy && n.1 != assetname);
        }
        debug!("Reward Tokens after Vesting filte:\n'{:?}'", rwd_tokens);
        rwdtxd.set_reward_tokens(&rwd_tokens);
        if rwdtxd.get_reward_tokens().is_empty() {
            return Err(CmdError::Custom {
                str: "The requested tokens are all still in a vesting period.".to_string(),
            }
            .into());
        }

        info!("check reward balances...");
        //Check token balances
        for token in rwdtxd.get_reward_tokens() {
            let fingerprint = murin::chelper::make_fingerprint(
                &hex::encode(token.0.to_bytes()),
                &hex::encode(token.1.name()),
            )?;
            info!("Fingerprint: '{}'", fingerprint);
            if token.2.compare(&murin::clib::utils::to_bignum(0)) <= 0 {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR claiming amount '0' of a token is not allowed: '{:?}'",
                        fingerprint
                    ),
                }
                .into());
            }
            info!("Try to get available rewards!");
            log::debug!("{:?}", contract);
            log::debug!("{:?}", fingerprint);
            log::debug!("{:?}", murin::clib::utils::from_bignum(&token.2) as i64);
            log::debug!(
                "{:?}",
                gtxd.get_stake_address()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for stake address")
            );
            log::debug!("{:?}", self.customer_id() as i64);
            match gungnir::Rewards::get_available_rewards(
                &gcon,
                &gtxd
                    .get_stake_address()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for stake address"),
                &rwdtxd
                    .get_payment_addr()
                    .to_bech32(None)
                    .expect("ERROR Could not construct bech32 address for payment address"),
                &fingerprint,
                contract.contract_id as i64,
                self.customer_id() as i64,
                murin::clib::utils::from_bignum(&token.2) as i64,
            )? {
                i if i >= 0 => {
                    info!(
                        "User has enough tokens earned to claim: '{:?}'",
                        fingerprint
                    );
                }
                _ => {
                    info!("Error in Rewards!");
                    return Err(CmdError::Custom {
                        str: format!(
                            "ERROR user did not earned enough tokens to claim this amount: '{:?}'",
                            token
                        ),
                    }
                    .into());
                }
            }
        }

        info!("lookup additional data...");
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;

        if let Some(fee_addr) = keyloc.fee_wallet_addr {
            if let Ok(addr) = murin::clib::address::Address::from_bech32(&fee_addr) {
                rwdtxd.set_fee_wallet_addr(&addr);
            }
        };
        if let Some(fee) = keyloc.fee {
            rwdtxd.set_fee(&(fee as u64));
        };

        let ns_addr = contract.address;
        let ns_script = contract.plutus;
        let ns_version = contract.version.to_string();

        info!("retrieve blockchain data...");
        let dbsync = mimir::establish_connection()?;
        let slot = mimir::get_slot(&dbsync)?;
        gtxd.set_current_slot(slot as u64);

        let reward_wallet_utxos = mimir::get_address_utxos(&dbsync, &ns_addr)?;
        if reward_wallet_utxos.is_empty() {
            // ToDO :
            // Send Email to Admin that not enough tokens are available on the script
            return Err(CmdError::Custom {
                str: "The contract does not contain enough tokens to claim, please try again later"
                    .to_string(),
            }
            .into());
        }

        rwdtxd.set_reward_utxos(&Some(reward_wallet_utxos.clone()));

        //For Debugging:  Lookup specific transaction hash in input utxos
        /*let myp = reward_wallet_utxos.find_utxo_by_txhash(&"a739902b7bc0eca2aabb431f8ddbe17c2d4de560c1b025321dcd2855b0660305".to_string(), 1);
        match myp {
            Some(y) => {
                info!("The UTXO: {:?}",reward_wallet_utxos.get(y));
            }
            None => {
                info!("Could not find the Utxo :(")
            }
        }
        */

        //ToDO:
        // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
        let ident = crate::encryption::mident(
            &contract.user_id,
            &contract.contract_id,
            &contract.version,
            &ns_addr,
        );
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
        info!("build transaction...");
        let bld_tx = murin::rwdist::build_rwd_multisig(
            &gtxd,
            &rwdtxd,
            &pkvs,
            &ns_addr,
            &ns_script,
            &ns_version,
        )
        .await?;

        info!("post processing transaction...");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &rwdtxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &contract.contract_id,
            &contract.version,
        );
        debug!("RAWTX data: {:?}", tx);

        info!("create response...");
        let ret = super::create_response(
            &bld_tx,
            &tx,
            self.transaction_pattern().wallet_type().as_ref(),
        )?;

        Ok(ret.to_string())
    }

    async fn handle_collection_mint(&self) -> crate::Result<String> {
        match self
            .transaction_pattern()
            .script()
            .ok_or("ERROR: No specific contract data supplied")?
        {
            ScriptSpecParams::NftMinter {
                receiver_stake_addr,
                receiver_payment_addr,
                ..
            } => {
                let err = Err(CmdError::Custom {
                    str: format!(
                        "ERROR wrong data provided for script specific parameters: '{:?}'",
                        self.transaction_pattern().script()
                    ),
                }
                .into());
                if murin::decode_addr(&receiver_payment_addr).await.is_err() {
                    return err;
                } else if let Some(saddr) = receiver_stake_addr {
                    if murin::wallet::get_stake_address(&murin::decode_addr(&saddr).await?)?
                        != murin::wallet::get_stake_address(
                            &murin::decode_addr(&receiver_payment_addr).await?,
                        )?
                    {
                        return err;
                    }
                }
            }
            _ => {
                return Err(CmdError::Custom {
                    str: format!("ERROR wrong data provided for '{:?}'", self.multisig_type()),
                }
                .into());
            }
        }
        log::debug!("Checks okay...");

        log::debug!("Try to create raw data...");
        let mut minttxd = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_mintdata()
            .await?;
        let mut gtxd = self.transaction_pattern().into_txdata().await?;

        log::debug!("Check contract...");
        let contract = determine_contract(gtxd.get_contract_id(), self.customer_id as i64)?
            .expect("Could not find valid contract");

        log::debug!("Try to establish database connection...");
        let drasildbcon = crate::database::drasildb::establish_connection()?;

        log::debug!("Try to determine additional data...");
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;

        log::debug!("Try to determine fees...");
        if let Some(feeaddr) = keyloc.fee_wallet_addr {
            match murin::b_decode_addr(&feeaddr).await {
                Ok(a) => minttxd.set_fee_addr(a),
                Err(e) => {
                    return Err(CmdError::Custom {
                        str: format!("ERROR could not decode fee address: '{:?}'", e.to_string()),
                    }
                    .into());
                }
            };
        };

        if let Some(fee) = keyloc.fee {
            minttxd.set_fee(fee)
        };

        let ns_addr: Option<String> = Some(contract.address.clone());
        // ToDo:
        //if minttxd.get_to_vendor_script() == true {
        //    ns_addr = Some(contract.address);
        //}
        let ns_script = contract.plutus;
        let ns_version = contract.version.to_string();

        log::debug!("Try to determine nft data...");
        let gcon = gungnir::establish_connection()?;
        let mint_project = gungnir::MintProject::get_mintproject_by_uid_cid(
            &gcon,
            self.customer_id as i64,
            contract.contract_id,
        )?;
        let _whitelist: Option<gungnir::Whitelist>;
        if let Some(wid) = mint_project.whitelist_id {
            _whitelist = Some(gungnir::Whitelist::get_whitelist(&gcon, wid)?);
        } else {
            _whitelist = None
        };

        let policy_id = contract
            .policy_id
            .expect("Error: The provided contract is not eligable to mint, no policy ID");
        let payment_addr_bech32 = minttxd.get_payment_addr_bech32()?;

        let mut nfts_to_mint = Vec::<gungnir::Nft>::new();
        let mut avail_rewards = Vec::<gungnir::Rewards>::new();
        let mut eligable_nfts = Vec::<gungnir::Nft>::new();
        if mint_project.reward_minter {
            nfts_to_mint =
                gungnir::Nft::get_nft_by_payaddr(&gcon, mint_project.id, &payment_addr_bech32)?;
            nfts_to_mint.retain(|n| !n.minted);
            if nfts_to_mint.is_empty() {
                return Err(CmdError::Custom {
                    str: "ERROR No assets for this address available".to_string(),
                }
                .into());
            }
            for nft in &nfts_to_mint {
                let fingerprint = murin::chelper::make_fingerprint(&policy_id, &nft.asset_name)?;
                let rewards = gungnir::Rewards::get_avail_specific_asset_reward(
                    &gcon,
                    &payment_addr_bech32,
                    &murin::cip30::get_bech32_stake_address_from_str(&payment_addr_bech32)?,
                    &fingerprint,
                    contract.contract_id,
                    self.customer_id(),
                )?;
                if !rewards.is_empty() {
                    eligable_nfts.push(nft.clone());
                }
                avail_rewards.extend(rewards.iter().map(|n| n.to_owned()));
            }
            if avail_rewards.len() != eligable_nfts.len() {
                return Err(CmdError::Custom{str:"ERROR there is some missmatch in your available rewards please contact support".to_string()}.into());
            }
        } else {
            let claimed = gungnir::Claimed::get_claims(
                &gcon,
                &murin::cip30::get_bech32_stake_address_from_str(&payment_addr_bech32)?,
                contract.contract_id,
                self.customer_id(),
            )?;
            //ToDo: Add parameter for NFTs minted per transaction
            let mut max_nfts = 1;
            if let Some(max) = mint_project.max_mint_p_addr {
                if claimed.len() >= max as usize {
                    return Err(CmdError::Custom {
                        str: "ERROR No assets for this address available".to_string(),
                    }
                    .into());
                }
                max_nfts = max;
            }
            for _ in 0..max_nfts {
                nfts_to_mint.extend(
                    gungnir::Nft::get_random_unminted_nft(&gcon, mint_project.id)?.into_iter(),
                )
            }
            //eligable_nfts = nfts_to_mint.clone();
        }

        // create MintTokenAsset datatype for all nfts to be minted
        minttxd.set_mint_tokens(convert_nfts_to_minter_token_asset(
            &nfts_to_mint,
            &policy_id,
        )?);

        // create transaction specific metadata
        let mut metadataassets = Vec::<murin::minter::AssetMetadata>::new();
        for nft in &nfts_to_mint {
            metadataassets.push(serde_json::from_str(&nft.metadata)?)
        }
        let metadata = murin::minter::Cip25Metadata {
            assets: metadataassets,
            other: None,
            version: "1.0".to_string(),
        };
        minttxd.set_metadata(metadata);

        log::debug!("Try to determine slot...");
        let dbsync = match mimir::establish_connection() {
            Ok(conn) => conn,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
                }
                .into());
            }
        };
        let slot = match mimir::get_slot(&dbsync) {
            Ok(s) => s,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR could not determine current slot: '{:?}'",
                        e.to_string()
                    ),
                }
                .into());
            }
        };
        gtxd.set_current_slot(slot as u64);

        //ToDO:
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd

        let ident = crate::encryption::mident(
            &contract.user_id,
            &contract.contract_id,
            &contract.version,
            &contract.address,
        );
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;

        log::debug!("Try to build transaction...");
        let bld_tx = murin::minter::build_mint_multisig(
            &gtxd,
            &minttxd,
            &pkvs,
            ns_addr.as_ref(),
            &ns_script,
            &ns_version,
        )
        .await?;
        log::debug!("Try to create raw tx...");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &minttxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &contract.contract_id,
            &contract.version,
        );
        debug!("RAWTX data: {:?}", tx);

        log::debug!("Try to create response data...");
        let ret = super::create_response(
            &bld_tx,
            &tx,
            self.transaction_pattern().wallet_type().as_ref(),
        )?;

        Ok(ret.to_string())
    }

    async fn handle_onehshot_mint(&self) -> crate::Result<String> {
        log::debug!("Entered Oneshot Minter...");
        let minttxd = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_mintdata()
            .await?;
        log::debug!("Minter Txd: {:?}", minttxd);
        let mut txp = self.transaction_pattern();
        txp.set_sending_wal_addrs(&[minttxd.get_payment_addr_bech32()?]);
        log::debug!("Transaction Patter: {:?}\n", &txp);
        log::debug!("Try to create general transaction data...");
        let mut gtxd = txp.into_txdata().await?;
        log::debug!("Connect to dbsync...");
        let dbsync = match mimir::establish_connection() {
            Ok(conn) => conn,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
                }
                .into());
            }
        };
        log::debug!("Get Slot...");
        let slot = match mimir::get_slot(&dbsync) {
            Ok(s) => s,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR could not determine current slot: '{:?}'",
                        e.to_string()
                    ),
                }
                .into());
            }
        };
        gtxd.set_current_slot(slot as u64);

        log::debug!("Create Oneshot policy...");
        let oneshotwallet = murin::wallet::create_wallet();
        let oneshotpolicy = murin::minter::create_onshot_policy(&oneshotwallet.3, slot as u64);

        log::debug!("Connect to platform db...");
        let drasildbcon = crate::database::drasildb::establish_connection()?;
        log::debug!("Check contract...");
        let contract = TBContracts::get_liquidity_wallet(&self.customer_id())?;
        log::debug!("Try to determine additional data...");
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;
        let ident = crate::encryption::mident(
            &contract.user_id,
            &contract.contract_id,
            &contract.version,
            &contract.address,
        );
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
        let ns_script = oneshotpolicy.0;

        //ToDO:
        //
        // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd

        log::debug!("Set utxos for input...");
        gtxd.set_inputs(mimir::get_address_utxos(&dbsync, &contract.address)?);

        log::debug!("Try to build transactions...");
        let bld_tx = match murin::minter::build_oneshot_mint::build_oneshot_mint_multisig(
            &gtxd,
            &minttxd,
            &pkvs,
            &ns_script,
            &murin::cip30::b_decode_addr(&contract.address).await?,
        )
        .await
        {
            Ok(o) => o,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not build transaction: '{:?}'", e.to_string()),
                }
                .into());
            }
        };

        log::debug!("Try to create Raw Tx...");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &minttxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &(-1),
            &(0.1),
        );

        log::debug!("Finalize...");
        let used_utxos = tx.get_usedutxos().clone();
        let txh = murin::finalize_rwd(
            &hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()),
            tx,
            vec!["".to_string(), hex::encode(oneshotwallet.0.as_bytes())],
        )
        .await?;

        log::debug!("Store used utxos...");
        murin::utxomngr::usedutxos::store_used_utxos(
            &txh,
            &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
        )?;

        let mut tokennames = Vec::<String>::new();
        let mut amounts = Vec::<u64>::new();
        let policy_id = hex::encode(ns_script.hash().to_bytes());

        for t in minttxd.get_mint_tokens() {
            tokennames.push(hex::encode(t.1.name()));
            amounts.push(murin::clib::utils::from_bignum(&t.2));
        }

        let ret = OneShotReturn::new(
            &policy_id,
            &tokennames,
            &amounts,
            &txh,
            &bld_tx.get_metadata(),
        );

        Ok(json!(ret).to_string())
    }

    async fn handle_testrewards(&self) -> crate::Result<String> {
        log::info!("Handle Testreward ....");
        let mut minttxd = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_mintdata()
            .await?;
        log::info!("Convert General Transaction Data ....");
        let mut gtxd = self.transaction_pattern().into_txdata().await?;

        log::info!("Determine network....");
        if gtxd.get_network() != murin::clib::NetworkIdKind::Testnet {
            return Err(CmdError::Custom {
                str: "ERROR: this functions is just for testing".to_string(),
            }
            .into());
        }
        log::info!("Randomize....");
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_entropy();
        let i: usize = rng.gen_range(0..3); // random number

        let mut t1 = Vec::<murin::MintTokenAsset>::new();
        let mta1: murin::txbuilders::MintTokenAsset = (
            None,
            murin::clib::AssetName::new("ttFLZC".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(138),
        );
        let mta2: murin::txbuilders::MintTokenAsset = (
            None,
            murin::clib::AssetName::new("ttSIL".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(142),
        );
        let mta3: murin::txbuilders::MintTokenAsset = (
            None,
            murin::clib::AssetName::new("ttDRSL".as_bytes().to_vec()).unwrap(),
            murin::clib::utils::to_bignum(63),
        );
        t1.push(mta1);
        t1.push(mta2);
        t1.push(mta3);

        let mut metadataarray = Vec::<String>::new();
        let m1 = "{\"assets\":[{\"name\":\"ttFLZC\",\"tokenname\":\"tFLZC\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string();
        let m2 = "{\"assets\":[{\"name\":\"ttSIL\",\"tokenname\":\"tSIL\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string();
        let m3 = "{\"assets\":[{\"name\":\"ttDRSL\",\"tokenname\":\"tDRSL\",\"mediaType\":\"image/png\",\"descritpion\":[\"MyDescription\"],\"image_url\":\"nourl\",\"files\":[],\"other\":[]}],\"version\":\"1.0\"}".to_string();
        metadataarray.push(m1);
        metadataarray.push(m2);
        metadataarray.push(m3);

        let tns = vec![
            hex::encode("tFLZC".as_bytes()),
            hex::encode("tSIL".as_bytes()),
            hex::encode("tDRSL".as_bytes()),
        ];
        let tokens = vec![t1[i].clone()];

        let t_minter_contract_id = 111;
        let t_minter_user_id = 111;
        let sporwc_tcontract_id = 1;
        let sporwc_user_id = 0;

        log::info!("Created raw data!");
        let drasildbcon = crate::database::drasildb::establish_connection()?;
        log::info!("Established connection!!");
        let contract = crate::drasildb::TBContracts::get_contract_uid_cid(
            t_minter_user_id,
            t_minter_contract_id,
        )?;

        let sporwc_flz = crate::drasildb::TBContracts::get_contract_uid_cid(
            sporwc_user_id,
            sporwc_tcontract_id,
        )?;

        log::info!("Got contract!");
        let contract_address = murin::address::Address::from_bech32(&contract.address).unwrap();
        let _sporwc_cadress = murin::address::Address::from_bech32(&sporwc_flz.address).unwrap();
        let rnd_mintdata = murin::minter::MinterTxData::new(
            tokens,
            None,
            contract_address.clone(), //reward contract address
            serde_json::from_str(&metadataarray[i])
                .expect("ERROR: Could not deserialize metadata in testrewards method"),
            true, //auto_mint
            None,
            None,
            t_minter_contract_id,
        );

        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;
        log::info!("Drasil Connection!");
        log::info!("keyloc: {:?}", keyloc);

        if let Some(saddr) = keyloc.fee_wallet_addr {
            match murin::b_decode_addr(&saddr).await {
                Ok(a) => minttxd.set_fee_addr(a),
                Err(e) => {
                    return Err(CmdError::Custom {
                        str: format!(
                            "ERROR could not decode signers address: '{:?}'",
                            e.to_string()
                        ),
                    }
                    .into());
                }
            };
        };

        let minttxd = rnd_mintdata;

        let ns_addr: Option<String> = Some(sporwc_flz.address);
        // ToDo:
        //if minttxd.get_to_vendor_script() == true {
        //    ns_addr = Some(contract.address);
        //}
        let ns_script = contract.plutus.clone();
        let ns_version = contract.version.to_string();

        let dbsync = match mimir::establish_connection() {
            Ok(conn) => conn,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
                }
                .into());
            }
        };
        let slot = match mimir::get_slot(&dbsync) {
            Ok(s) => s,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR could not determine current slot: '{:?}'",
                        e.to_string()
                    ),
                }
                .into());
            }
        };
        gtxd.set_current_slot(slot as u64);
        log::info!("DB Sync Slot: {}", slot);
        //ToDO:
        // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
        let policy_script_utxos = mimir::get_address_utxos(&dbsync, &contract.address)?;

        gtxd.set_inputs(policy_script_utxos);

        let ident = crate::encryption::mident(
            &contract.user_id,
            &contract.contract_id,
            &contract.version,
            &contract.address,
        );
        let pkvs = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
        let bld_tx = murin::minter::build_mint_multisig(
            &gtxd,
            &minttxd,
            &pkvs,
            ns_addr.as_ref(),
            &ns_script,
            &ns_version,
        )
        .await?;
        info!("Build Successful!");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &minttxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &contract.contract_id,
            &contract.version,
        );
        debug!("RAWTX data: {:?}", tx);
        let used_utxos = tx.get_usedutxos().clone();
        let txh = murin::finalize_rwd(
            &hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()),
            tx,
            pkvs,
        )
        .await?;
        murin::utxomngr::usedutxos::store_used_utxos(
            &txh,
            &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
        )?;

        use gungnir::FromPrimitive;

        // Create reward entry
        let gconn = gungnir::establish_connection()?;
        let policy = hex::encode(
            murin::clib::NativeScript::from_bytes(hex::decode(contract.plutus).unwrap())
                .unwrap()
                .hash()
                .to_bytes(),
        );
        let fingerprint = murin::make_fingerprint(&policy, &tns[i])?;
        let rewards = gungnir::Rewards::get_rewards_per_token(
            &gconn,
            &gtxd.get_stake_address().to_bech32(None).unwrap(),
            sporwc_flz.contract_id as i64,
            sporwc_flz.user_id as i64,
            &fingerprint,
        )?;
        let current_epoch = mimir::get_epoch(&mimir::establish_connection()?)? as i64;
        println!("Rewards: {:?}", rewards);
        if rewards.len() == 1 && rewards[0].fingerprint == fingerprint {
            let tot_earned = rewards[0].tot_earned.clone()
                + (gungnir::BigDecimal::from_u64(murin::clib::utils::from_bignum(
                    &minttxd.get_mint_tokens()[0].2,
                ))
                .unwrap()
                    * gungnir::BigDecimal::from_u64(1000000).unwrap());

            let stake_rwd = gungnir::Rewards::update_rewards(
                &gconn,
                &rewards[0].stake_addr,
                &rewards[0].fingerprint,
                &rewards[0].contract_id,
                &rewards[0].user_id,
                &tot_earned,
                &current_epoch,
            );
            println!("Stake Rewards Update: {:?}", stake_rwd);
        }
        if rewards.is_empty() {
            let tot_earned = gungnir::BigDecimal::from_u64(murin::clib::utils::from_bignum(
                &minttxd.get_mint_tokens()[0].2,
            ))
            .unwrap()
                * gungnir::BigDecimal::from_u64(1000000).unwrap();

            let stake_rwd = gungnir::Rewards::create_rewards(
                &gconn,
                &gtxd.get_stake_address().to_bech32(None).unwrap(),
                &mimir::api::select_addr_of_first_transaction(
                    &gtxd
                        .get_stake_address()
                        .to_bech32(None)
                        .expect("ERROR Could not construct bech32 address for stake address"),
                )?,
                &fingerprint,
                &(sporwc_flz.contract_id as i64),
                &(sporwc_flz.user_id as i64),
                &tot_earned,
                &gungnir::BigDecimal::from_i32(0).unwrap(),
                &true,
                &current_epoch,
            );
            println!("Stake Rewards New: {:?}", stake_rwd);
        }

        let ret = "Creating Rewards for you was successfull";
        Ok(ret.to_string())
    }

    async fn handle_customer_payout(&self) -> crate::Result<String> {
        info!("verify transaction data...");
        // ToDo:
        // Verify there is a unhandled payout existing for this user with the security code passed in cpo_data,
        // Payout need to be verified and approved by a DrasilAdmin (In the best case after creation and signature of the customer)
        // The Drasil verification would apply the last needed MultiSig Key for the payout so no accidential payout is possible.

        info!("create raw data...");
        let cpo_data = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_cpo()
            .await?;
        let mut gtxd = self.transaction_pattern().into_txdata().await?;

        info!("establish database connections...");
        let drasildbcon = crate::database::drasildb::establish_connection()?;

        let contract = crate::drasildb::TBContracts::get_contract_uid_cid(
            cpo_data.get_user_id(),
            cpo_data.get_contract_id(),
        )?;

        let _contract_address = murin::address::Address::from_bech32(&contract.address).unwrap();

        info!("retrieve additional data...");
        let keyloc = crate::drasildb::TBMultiSigLoc::get_multisig_keyloc(
            &drasildbcon,
            &contract.contract_id,
            &contract.user_id,
            &contract.version,
        )?;
        info!("Drasil Connection!");
        info!("keyloc: {:?}", keyloc);

        let _ns_script = contract.plutus.clone();
        let _ns_version = contract.version.to_string();

        let dbsync = match mimir::establish_connection() {
            Ok(conn) => conn,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!("ERROR could not connect to dbsync: '{:?}'", e.to_string()),
                }
                .into());
            }
        };
        let slot = match mimir::get_slot(&dbsync) {
            Ok(s) => s,
            Err(e) => {
                return Err(CmdError::Custom {
                    str: format!(
                        "ERROR could not determine current slot: '{:?}'",
                        e.to_string()
                    ),
                }
                .into());
            }
        };
        gtxd.set_current_slot(slot as u64);

        // ToDO:
        // Determine Available Payout Sum and write it into cpo_data so the txbuild can create correct transaction
        // The Sum is determined automatically by: Outputsum = Ada_Available_on_contract - (Total_Liquidity)
        // make sure no tokens are leaving the contract (possibly a rearrangement of Utxos is needed before and after the payout?)

        // - Function to check and split utxos when for size >5kB (cal_min_ada panics on utxos >5kB)
        // - Find a solution for protocal parameters (maybe to database?) at the moment they are hardcoded in list / build_rwd
        let contract_utxos = mimir::get_address_utxos(&dbsync, &contract.address)?;

        gtxd.set_inputs(contract_utxos);

        /*
        let bld_tx = murin::rwdist::build_cpo(&gtxd, &cpo_data, &keyloc.pvks, ns_addr.as_ref(), &ns_script, &ns_version).await?;
        info!("Build Successful!");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(),
            &bld_tx.get_txwitness(),
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(),
            &minttxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &contract.contract_id,
            &contract.version,
        );
        debug!("RAWTX data: {:?}",tx);
        let used_utxos = tx.get_usedutxos().clone();
        let txh = murin::finalize_rwd(&hex::encode(&murin::clib::TransactionWitnessSet::new().to_bytes()), tx, keyloc.pvks).await?;
        murin::utxomngr::usedutxos::store_used_utxos(&txh, &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?)?;

        let ret = super::create_response(&bld_tx, &tx, self.transaction_pattern().wallet_type().as_ref())?;
        */
        let ret = "Not implemented";
        Ok(ret.to_string())
    }
}

impl IntoFrame for BuildMultiSig {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("bms".as_bytes()));

        frame.push_int(self.customer_id);

        let mtype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.mtype)
            .unwrap();
        frame.push_bulk(Bytes::from(mtype_b));

        let txpattern_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.txpattern)
            .unwrap();
        frame.push_bulk(Bytes::from(txpattern_b));

        frame
    }
}

pub fn determine_contract(
    contract_id: Option<u64>,
    customer_id: i64,
) -> Result<Option<crate::drasildb::TBContracts>, MurinError> {
    let u_customer_id = customer_id;
    if let Some(contract_id) = contract_id {
        log::debug!("Get defined contract {:?}...", contract_id);
        let u_contract_id = contract_id as i64;
        log::debug!("Lookup Data: User: {:?}, ", (u_customer_id));
        log::debug!("Lookup Data: Contract ID: {:?}, ", (contract_id as i64));
        let tcontract =
            crate::drasildb::TBContracts::get_contract_uid_cid(u_customer_id, u_contract_id);
        log::debug!("Found contract: {:?}...", tcontract);
        Ok(Some(tcontract?))
    } else {
        Ok(None)
    }
}

pub fn convert_nfts_to_minter_token_asset(
    nfts: &Vec<gungnir::Nft>,
    policy_id: &String,
) -> Result<Vec<murin::MintTokenAsset>, MurinError> {
    let mut out = Vec::<murin::MintTokenAsset>::new();
    for nft in nfts {
        out.push((
            Some(murin::chelper::string_to_policy(policy_id)?),
            murin::chelper::string_to_assetname(&nft.asset_name)?,
            murin::u64_to_bignum(1),
        ))
    }
    Ok(out)
}
