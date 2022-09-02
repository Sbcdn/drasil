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
pub use verifyuser::VerifyUser;

mod getstakekey;
pub use getstakekey::GetStakeKey;

mod verifydata;
pub use verifydata::VerifyData;

mod unknown;
pub use unknown::Unknown;

mod error;
pub use error::CmdError;

use crate::{Connection, Frame, Parse, Shutdown, TransactionPattern};

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
    GetStakeKey(GetStakeKey),
    VerifyData(VerifyData),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;

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
            "gsk" => {
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
            Command::GetStakeKey(_) => "gsk",
            Command::VerifyData(_) => "vd",
            Command::Unknown(_) => "unkw",
        }
    }
}

// ToDO: Make different pattern checks for API | MultiSIg | Smart Contract
async fn check_txpattern(txp: &TransactionPattern) -> crate::Result<()> {
    let empty_vec = Vec::<String>::new();
    // Check if user is valid
    let _username = txp.user();

    // TODO
    if txp.inputs().is_none() || txp.inputs().unwrap_or(empty_vec).is_empty() {
        // Get inputs from dbsync and format them to TransactionUnspentOutputs (byte encoded or not ? )
    }

    if txp.outputs().is_some() {
        return Err(CmdError::Custom {
            str: "ERROR outputs must be None for this contract type".to_string(),
        }
        .into());
    }

    if txp.collateral().is_some() && //txp.collateral().is_none() ||
         (hex::decode(txp.collateral().unwrap()).is_err() || txp.collateral().unwrap() == "")
    {
        return Err(CmdError::Custom {
            str: "ERROR no collateral provided".to_string(),
        }
        .into());
    }

    if txp.sending_wal_addrs().is_empty() {
        return Err(CmdError::Custom {
            str: "ERROR no wallet address provided".to_string(),
        }
        .into());
    }

    if txp.sending_stake_addr().is_some() {
        let addresses = murin::cip30::decode_addresses(&txp.sending_wal_addrs()).await?;
        let stake_addr = murin::cip30::decode_addr(&txp.sending_stake_addr().unwrap()).await?;
        let stake_addr_hash = murin::cip30::get_stake_address(&stake_addr)?.to_bytes();
        for address in addresses {
            let s_addr_hash = murin::cip30::get_stake_address(&address)?.to_bytes();

            if s_addr_hash != stake_addr_hash {
                return Err(CmdError::Custom{str:"ERROR stake address does not match one of the provided addresses, beware manipulation!".to_string()}.into());
            }
        }
    }
    log::debug!("TxPattern okay!");
    Ok(())
}

pub fn create_response(
    bld_tx: &murin::htypes::BuildOutput,
    raw_tx: &murin::utxomngr::RawTx,
    wallet_type: Option<&crate::datamodel::hephadata::WalletType>,
) -> Result<crate::datamodel::hephadata::UnsignedTransaction, murin::MurinError> {
    debug!("RawTx: {:?}", raw_tx);
    let tx_id = murin::utxomngr::txmind::store_raw_tx(raw_tx)?;
    let mut response = crate::datamodel::hephadata::UnsignedTransaction::new(
        Some(&bld_tx.get_tx_unsigned()),
        &tx_id,
    );

    if let Some(wallet) = wallet_type {
        if *wallet == crate::datamodel::hephadata::WalletType::Yoroi {
            response.set_tx(&bld_tx.get_tx_unsigned())
        }
    }

    Ok(response)
}
