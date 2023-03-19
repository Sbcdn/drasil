/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
mod buildcontract;
pub use buildcontract::BuildContract;

mod buildmultisig;
pub use buildmultisig::BuildMultiSig;

pub mod buildtx;
pub use buildtx::BuildStdTx;

mod finalizecontract;
pub use finalizecontract::FinalizeContract;

mod finalizemultisig;
pub use finalizemultisig::FinalizeMultiSig;

mod finalizestdtx;
pub use finalizestdtx::FinalizeStdTx;

mod verifyuser;
use murin::address::BaseAddress;
use murin::get_reward_address;
pub use verifyuser::VerifyUser;

mod hydra;
pub use hydra::HydraOps;

mod verifydata;
pub use verifydata::VerifyData;

mod unknown;
pub use unknown::Unknown;

mod error;
pub use error::CmdError;

mod discount;
use crate::error::SystemDBError;
use crate::{Connection, Frame, Parse, Shutdown, TransactionPattern};
pub(crate) use discount::*;

pub trait IntoFrame {
    fn into_frame(self) -> Frame;
}

#[derive(Debug)]
pub enum Command {
    BuildContract(BuildContract),
    BuildMultiSig(BuildMultiSig),
    BuildStdTx(BuildStdTx),
    FinalizeContract(FinalizeContract),
    FinalizeMultiSig(FinalizeMultiSig),
    FinalizeStdTx(FinalizeStdTx),
    VerifyUser(VerifyUser),
    HydraOperation(HydraOps),
    VerifyData(VerifyData),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;
        log::debug!("FromFrame: {:?}", &parse);
        let command_name = parse.next_string()?.to_lowercase();

        let command: Command = match &command_name[..] {
            //Build Contract
            "bct" => Command::BuildContract(BuildContract::parse_frames(&mut parse)?),
            //Build MultiSig
            "bms" => Command::BuildMultiSig(BuildMultiSig::parse_frames(&mut parse)?),
            //Finalize Contract
            "stx" => Command::BuildStdTx(BuildStdTx::parse_frames(&mut parse)?),
            //Finalize Contract
            "fct" => Command::FinalizeContract(FinalizeContract::parse_frames(&mut parse)?),
            //Finalize MultiSig
            "fms" => Command::FinalizeMultiSig(FinalizeMultiSig::parse_frames(&mut parse)?),
            //Finalize MultiSig
            "ftx" => Command::FinalizeStdTx(FinalizeStdTx::parse_frames(&mut parse)?),
            //GetPubKey
            "vus" => Command::VerifyUser(VerifyUser::parse_frames(&mut parse)?),
            //GetStakeKey
            "hyd" => {
                Command::Unknown(Unknown::new(command_name))
                //Command::GetStakeKey(GetStakeKey::parse_frames(&mut parse)?)
            }
            //VerifyData
            "vd" => {
                Command::Unknown(Unknown::new(command_name))
                //Command::VerifyData(VerifyData::parse_frames(&mut parse)?)
            }
            _ => Command::Unknown(Unknown::new(command_name)),
        };

        parse.finish()?;

        Ok(command)
    }

    pub async fn apply(self, dst: &mut Connection, _shutdown: &mut Shutdown) -> crate::Result<()> {
        match self {
            Command::BuildContract(cmd) => cmd.apply(dst).await?,
            Command::BuildMultiSig(cmd) => cmd.apply(dst).await?,
            Command::BuildStdTx(cmd) => cmd.apply(dst).await?,
            Command::FinalizeContract(cmd) => cmd.apply(dst).await?,
            Command::FinalizeMultiSig(cmd) => cmd.apply(dst).await?,
            Command::FinalizeStdTx(cmd) => cmd.apply(dst).await?,
            Command::VerifyUser(cmd) => cmd.apply(dst).await?,
            Command::Unknown(cmd) => cmd.apply(dst).await?,

            _ => {
                let response = Frame::Simple("ERROR command could not be applied".to_string());
                debug!("{:?}", response);
                dst.write_frame(&response).await?;
            }
        }

        Ok(())
    }

    pub(crate) fn _get_name(&self) -> &str {
        match self {
            Command::BuildContract(_) => "bct",
            Command::BuildMultiSig(_) => "bms",
            Command::BuildStdTx(_) => "stx",
            Command::FinalizeContract(_) => "fct",
            Command::FinalizeMultiSig(_) => "fms",
            Command::FinalizeStdTx(_) => "ftx",
            Command::VerifyUser(_) => "vus",
            Command::HydraOperation(_) => "hyd",
            Command::VerifyData(_) => "vd",
            Command::Unknown(_) => "unkw",
        }
    }
}

async fn check_txpattern(txp: &TransactionPattern) -> crate::Result<()> {
    let empty_vec = Vec::<String>::new();
    // Check if user is valid
    let _username = txp.user();

    // TODO
    if txp.utxos().is_none() || txp.utxos().unwrap_or(empty_vec).is_empty() {
        // Get inputs from dbsync and format them to TransactionUnspentOutputs (byte encoded or not ? )
    }

    if txp.collateral().is_some()
        && (hex::decode(txp.collateral().unwrap()).is_err() || txp.collateral().unwrap() == "")
    {
        return Err(CmdError::Custom {
            str: "ERROR no collateral provided".to_string(),
        }
        .into());
    }

    if txp.used_addresses().is_empty() {
        return Err(CmdError::Custom {
            str: "ERROR no wallet address provided".to_string(),
        }
        .into());
    }

    if txp.stake_addr().is_some() {
        let addresses = murin::cip30::decode_addresses(&txp.used_addresses()).await?;
        let stake_addr = murin::cip30::decode_addr(&txp.stake_addr().unwrap()).await?;
        let mut rewardaddr = get_reward_address(&stake_addr)?;
        for address in addresses {
            if BaseAddress::from_address(&address).is_some() {
                let raddr = get_reward_address(&address)?;
                if raddr != rewardaddr {
                    return Err(CmdError::Custom{str:"ERROR stake address does not match one of the provided addresses, beware manipulation!".to_string()}.into());
                }
                rewardaddr = raddr
            }
        }
    }
    log::debug!("TxPattern okay!");
    Ok(())
}

pub fn create_response(
    bld_tx: &murin::htypes::BuildOutput,
    raw_tx: &murin::utxomngr::RawTx,
    wallet_type: Option<&crate::datamodel::models::WalletType>,
) -> Result<crate::datamodel::models::UnsignedTransaction, murin::MurinError> {
    debug!("Try to store raw tx...");
    let tx_id = murin::utxomngr::txmind::store_raw_tx(raw_tx)?;
    debug!("Try to create response...");
    let mut response =
        crate::datamodel::models::UnsignedTransaction::new(Some(&bld_tx.get_tx_unsigned()), &tx_id);
    debug!("Determine wallet specific settings...");
    if let Some(wallet) = wallet_type {
        if *wallet == crate::datamodel::models::WalletType::Yoroi {
            response.set_tx(&bld_tx.get_tx_unsigned())
        }
    }
    debug!("Sending response...");
    Ok(response)
}

pub fn determine_contracts(
    contract_id: Option<Vec<i64>>,
    customer_id: i64,
) -> Result<Option<Vec<crate::drasildb::TBContracts>>, SystemDBError> {
    let u_customer_id = customer_id;
    if let Some(contract_id) = contract_id {
        log::debug!("Get defined contracts {:?}...", contract_id);
        let mut tcontracts = Vec::<crate::drasildb::TBContracts>::new();
        for cid in contract_id {
            tcontracts.push(crate::drasildb::TBContracts::get_contract_uid_cid(
                u_customer_id,
                cid,
            )?);
        }
        log::debug!("Found Contracts: {:?}", tcontracts);
        Ok(Some(tcontracts))
    } else {
        Ok(None)
    }
}

pub fn convert_nfts_to_minter_token_asset(
    nfts: &Vec<gungnir::minting::models::Nft>,
    policy_id: &String,
) -> Result<Vec<murin::MintTokenAsset>, murin::MurinError> {
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
