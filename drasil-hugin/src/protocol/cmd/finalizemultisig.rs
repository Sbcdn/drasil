use crate::datamodel::MultiSigType;
use crate::{CmdError, Parse, TBContracts};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use drasil_gungnir::models::{MintProject, MintReward, Nft};
use drasil_murin::make_fingerprint;
use drasil_murin::minter::models::{CMintHandle, ColMinterTxData};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct FinalizeMultiSig {
    customer_id: u64,
    mtype: MultiSigType,
    tx_id: String,
    signature: String,
}

impl IntoFrame for FinalizeMultiSig {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("fms".as_bytes()));

        frame.push_int(self.customer_id);

        let mtype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.mtype)
            .unwrap();
        frame.push_bulk(Bytes::from(mtype_b));

        frame.push_bulk(Bytes::from(self.get_tx_id().into_bytes()));

        frame.push_bulk(Bytes::from(self.get_signature().into_bytes()));

        frame
    }
}

impl FinalizeMultiSig {
    pub fn new(
        customer_id: u64,
        mtype: MultiSigType,
        tx_id: String,
        signature: String,
    ) -> FinalizeMultiSig {
        FinalizeMultiSig {
            customer_id,
            mtype,
            tx_id,
            signature,
        }
    }

    pub fn get_customer_id(&self) -> u64 {
        self.customer_id
    }

    pub fn get_contract_type(&self) -> MultiSigType {
        self.mtype.clone()
    }

    pub fn get_tx_id(&self) -> String {
        self.tx_id.clone()
    }

    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<FinalizeMultiSig> {
        let customer_id = parse.next_int()?;

        let mtype = parse.next_bytes()?;
        let mtype: MultiSigType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&mtype)?;

        let tx_id = parse.next_string()?;
        let tx_id = tx_id;

        let signature = parse.next_string()?;
        let signature = signature;

        Ok(FinalizeMultiSig {
            customer_id,
            mtype,
            tx_id,
            signature,
        })
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response = Frame::Simple("Error: something went wrong".to_string());
        let raw_tx = drasil_murin::utxomngr::txmind::read_raw_tx(&self.get_tx_id())?;

        let mut ret = String::new();
        let used_utxos = raw_tx.get_usedutxos().clone();
        match self.mtype {
            MultiSigType::SpoRewardClaim => {
                if let Err(e) =
                    drasil_murin::rwdist::RWDTxData::from_str(raw_tx.get_tx_specific_rawdata())
                {
                    return Err(CmdError::Custom{str:format!("ERROR Invalid Transaction Data, this is not a reward distribution transaction, {:?}",e.to_string())}.into());
                };

                let tx_data = drasil_murin::TxData::from_str(raw_tx.get_txrawdata())?;
                let rwd_data = drasil_murin::RWDTxData::from_str(raw_tx.get_tx_specific_rawdata())?;

                let mut gcon = drasil_gungnir::establish_connection()?;

                for handle in rwd_data.get_rewards() {
                    let fingerprint = drasil_murin::chelper::make_fingerprint(
                        &hex::encode(handle.get_policy_id()?.to_bytes()),
                        &hex::encode(handle.get_assetname()?.name()),
                    )?;
                    drasil_gungnir::Claimed::create_claim(
                        &mut gcon,
                        &tx_data
                            .get_stake_address()
                            .to_bech32(None)
                            .expect("Could not construct bech32 address for stake address"),
                        &rwd_data
                            .get_payment_addr()
                            .to_bech32(None)
                            .expect("Could not construct bech32 address for payment address"),
                        &fingerprint,
                        &drasil_murin::clib::utils::from_bignum(&handle.get_amount()?),
                        &(handle.get_contract_id()),
                        &raw_tx.get_user_id()?,
                        &ret.clone(),
                        None,
                        None,
                    )?;
                    drasil_gungnir::Rewards::update_claimed(
                        &mut gcon,
                        &tx_data.get_stake_address().to_bech32(None).unwrap(),
                        &fingerprint,
                        &handle.get_contract_id(),
                        &raw_tx.get_user_id()?,
                        &drasil_murin::clib::utils::from_bignum(&handle.get_amount()?),
                    )?;
                }
            }
            MultiSigType::NftCollectionMinter => {
                if let Err(e) = drasil_murin::minter::models::ColMinterTxData::from_str(
                    raw_tx.get_tx_specific_rawdata(),
                ) {
                    return Err(CmdError::Custom{str:format!("ERROR Invalid Transaction Data, this is not a collection minter transaction, {:?}",e.to_string())}.into());
                };
                drasil_murin::TxData::from_str(raw_tx.get_txrawdata())?;
                let mint_data = ColMinterTxData::from_str(raw_tx.get_tx_specific_rawdata())?;
                log::debug!("mint_data: {:?}", mint_data);

                let mut handles = Vec::<(CMintHandle, MintReward, MintProject, TBContracts)>::new();
                for m in &mint_data.mint_handles {
                    let mrwd = MintReward::get_mintreward_by_id(m.id)?;
                    let p = MintProject::get_mintproject_by_id(m.project_id)?;
                    log::debug!("Mintproject found....");
                    let c = TBContracts::get_contract_uid_cid(p.user_id, p.mint_contract_id)?;
                    log::debug!("Mintcontract found....");
                    for a in mrwd.nft_ids.clone() {
                        let nft = Nft::get_nft_by_assetnameb(p.id, &p.nft_table_name, &a)?;
                        if nft.minted || mrwd.processed || mrwd.minted {
                            return Err(CmdError::Custom {
                                str: format!("ERROR mint already processed, {nft:?}"),
                            }
                            .into());
                        }
                    }
                    handles.push((m.to_owned(), mrwd.to_owned(), p.to_owned(), c.to_owned()));
                }

                ret = self.finalize_rwd(raw_tx.clone()).await?;

                //Todo: processed must happen before finalize rwd
                for h in &handles {
                    MintReward::process_mintreward(h.1.id, h.1.project_id, &h.1.pay_addr)?;
                    for a in h.1.nft_ids.clone() {
                        let fingerprint =
                            make_fingerprint(h.3.policy_id.as_ref().unwrap(), &hex::encode(a))
                                .unwrap();
                        log::debug!("Fingerprint: {:?}", fingerprint);
                        Nft::set_nft_minted(&h.2.id, &h.2.nft_table_name, &fingerprint, &ret)
                            .await?;
                    }
                }
            }
            MultiSigType::NftVendor => {}

            MultiSigType::DAOVoting => {}

            MultiSigType::VestingWallet => {}

            MultiSigType::UTxOpti => {
                if let Err(e) = crate::Utxopti::from_str(raw_tx.get_tx_specific_rawdata()) {
                    return Err(CmdError::Custom {
                        str: format!(
                            "ERROR Invalid Transaction Data, this is not UTxOpti transaction, {:?}",
                            e.to_string()
                        ),
                    }
                    .into());
                };
                ret = self.finalize_utxopti(raw_tx.clone()).await?;
            }

            MultiSigType::Mint => {
                if let Err(e) =
                    drasil_murin::minter::MinterTxData::from_str(raw_tx.get_tx_specific_rawdata())
                {
                    return Err(CmdError::Custom {
                        str: format!(
                            "ERROR Invalid Transaction Data, this is not mint transaction, {:?}",
                            e.to_string()
                        ),
                    }
                    .into());
                };
                ret = self.finalize_mint(raw_tx.clone()).await?;
            }

            _ => {
                log::debug!("{:?}", response);
                dst.write_frame(&response).await?;
                return Ok(());
            }
        }

        drasil_murin::utxomngr::usedutxos::store_used_utxos(
            &ret,
            &drasil_murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
        )?;

        // ToDO:
        // store tx into permanent storage (drasildb)
        // delete build_tx from redis

        response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret)?,
        ));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }

    async fn finalize_rwd(&self, raw_tx: drasil_murin::RawTx) -> crate::Result<String> {
        use crate::database::drasildb::*;
        use drasil_murin::txbuilders::rwdist::finalize_rwd::finalize_rwd;
        let tx_data = drasil_murin::TxData::from_str(raw_tx.get_txrawdata())?;
        let mut pvks = Vec::<String>::new();
        log::debug!("TxData in Finalize: {:?}", tx_data);
        for cid in tx_data.get_contract_id().as_ref().unwrap() {
            log::debug!("Contract Ids:{:?}", tx_data.get_contract_id());
            let contract =
                crate::drasildb::TBContracts::get_contract_uid_cid(self.customer_id as i64, *cid)?;

            let keyloc = TBMultiSigLoc::get_multisig_keyloc(
                cid,
                &(self.customer_id as i64),
                &contract.version,
            )?;

            let ident = crate::encryption::mident(
                &contract.user_id,
                &contract.contract_id,
                &contract.version,
                &contract.address,
            );
            let _pvks = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
            pvks.push(_pvks[1].clone())
        }

        let response = finalize_rwd(&self.get_signature(), raw_tx, pvks).await?;
        info!("Response: {}", response);
        Ok(response)
    }

    async fn finalize_mint(&self, raw_tx: drasil_murin::RawTx) -> crate::Result<String> {
        use crate::database::drasildb::*;
        use drasil_murin::txbuilders::rwdist::finalize_rwd::finalize_rwd;

        let tx_data = drasil_murin::TxData::from_str(raw_tx.get_txrawdata())?;
        let mut pvks = Vec::<String>::new();

        for cid in tx_data.get_contract_id().as_ref().unwrap() {
            let contract =
                crate::drasildb::TBContracts::get_contract_uid_cid(self.customer_id as i64, *cid)?;

            let keyloc = TBMultiSigLoc::get_multisig_keyloc(
                cid,
                &(self.customer_id as i64),
                &contract.version,
            )?;

            let ident = crate::encryption::mident(
                &contract.user_id,
                &contract.contract_id,
                &contract.version,
                &contract.address,
            );
            let _pvks = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
            pvks.extend(_pvks.clone().iter().map(|n| n.to_owned()))
        }
        let response = finalize_rwd(&self.get_signature(), raw_tx, pvks).await?;
        info!("Response: {}", response);
        Ok(response)
    }

    async fn finalize_utxopti(&self, raw_tx: drasil_murin::RawTx) -> crate::Result<String> {
        use crate::database::drasildb::*;
        use drasil_murin::txbuilders::rwdist::finalize_utxopti::finalize_utxopti;

        let contract_ids = raw_tx.get_contract_id()?;
        let mut pvks = Vec::<String>::new();

        for cid in &contract_ids {
            let contract =
                crate::drasildb::TBContracts::get_contract_uid_cid(self.customer_id as i64, *cid)?;

            let keyloc = TBMultiSigLoc::get_multisig_keyloc(
                cid,
                &(self.customer_id as i64),
                &contract.version,
            )?;

            let ident = crate::encryption::mident(
                &contract.user_id,
                &contract.contract_id,
                &contract.version,
                &contract.address,
            );
            let _pvks = crate::encryption::decrypt_pkvs(keyloc.pvks, &ident).await?;
            for pk in _pvks {
                pvks.push(pk.clone())
            }
        }
        let response = finalize_utxopti(raw_tx, pvks).await?;
        info!("Response: {}", response);
        Ok(response)
    }
}
