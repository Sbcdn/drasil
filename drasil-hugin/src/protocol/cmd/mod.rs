mod buildcontract;
pub use buildcontract::BuildContract;

mod buildmultisig;
mod discount;
mod error;
mod finalizecontract;
mod finalizemultisig;
mod finalizestdtx;
mod unknown;
mod verifydata;
mod verifyuser;
pub use buildmultisig::BuildMultiSig;
pub use finalizecontract::FinalizeContract;
pub mod buildtx;
pub use buildtx::BuildStdTx;
pub(crate) use discount::*;
pub use error::CmdError;
pub use unknown::Unknown;
pub use verifydata::VerifyData;
pub use verifyuser::VerifyUser;

use drasil_murin::address::BaseAddress;
use drasil_murin::{cardano, wallet, MurinError};

use crate::error::SystemDBError;
use crate::{Connection, Frame, Parse, Shutdown, TransactionPattern};
pub use finalizemultisig::FinalizeMultiSig;
pub use finalizestdtx::FinalizeStdTx;

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
    VerifyData(VerifyData),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse =
            Parse::new(frame).map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let command_name = parse
            .next_string()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

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

        parse
            .finish()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

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
        return Err("ERROR no collateral provided".into());
    }

    if txp.used_addresses().is_empty() {
        match txp.operation() {
            Some(o) => match o {
                crate::Operation::SpoRewardClaim {
                    rewards: _,
                    recipient_stake_addr: _,
                    recipient_payment_addr: _,
                } => todo!(),
                crate::Operation::NftVendor {} => todo!(),
                crate::Operation::Marketplace {
                    tokens: _,
                    metadata: _,
                    royalties_addr: _,
                    royalties_rate: _,
                    selling_price: _,
                } => todo!(),
                crate::Operation::NftShop {
                    tokens: _,
                    metadata: _,
                    selling_price: _,
                } => todo!(),
                crate::Operation::Minter {
                    mint_tokens: _,
                    receiver_stake_addr: _,
                    receiver_payment_addr: _,
                    mint_metadata: _,
                    auto_mint: _,
                    contract_id: _,
                } => todo!(),
                crate::Operation::NftCollectionMinter { mint_handles: _ } => todo!(),
                crate::Operation::TokenMinter {} => todo!(),
                crate::Operation::NftOffer {
                    token: _,
                    token_owner_addr: _,
                    metadata: _,
                    royalties_addr: _,
                    royalties_rate: _,
                    offer_price: _,
                } => todo!(),
                crate::Operation::Auction {} => todo!(),
                crate::Operation::StakeDelegation {
                    poolhash: _,
                    addresses: _,
                } => todo!(),
                crate::Operation::StakeDeregistration {
                    payment_addresses: _,
                } => todo!(),
                crate::Operation::RewardWithdrawal {
                    withdrawal_amount: _,
                } => todo!(),
                crate::Operation::StdTx {
                    transfers: _,
                    wallet_addresses: wa,
                    //Todo: Remove unwrap
                } => {
                    if wa.unwrap().is_empty() {
                        return Err("ERROR no wallet addresses provided".into());
                    }
                }
                crate::Operation::WmtStaking {
                    ennft: _,
                    amount: _,
                } => {
                    todo!()
                }
                crate::Operation::WmEnRegistration {
                    datum: _,
                    wallet_addresses: wa,
                    stake_address: _,
                } => {
                    if wa.is_empty() {
                        return Err("ERROR no wallet addresses provided".into());
                    }
                }
                crate::Operation::CPO { po_id: _, pw: _ } => {}
                crate::Operation::ClApiOneShotMint {
                    tokennames: _,
                    amounts: _,
                    metadata: _,
                    receiver: _,
                } => {}
            },
            None => return Err("ERROR no wallet address provided".into()),
        }
    }

    if txp.stake_addr().is_some() {
        let addresses = wallet::addresses_from_string(&txp.used_addresses()).await?;
        let stake_addr = wallet::decode_address_from_bytes(&txp.stake_addr().unwrap()).await?;
        let mut rewardaddr = wallet::reward_address_from_address(&stake_addr)?;
        for address in addresses {
            if BaseAddress::from_address(&address).is_some() {
                let raddr = wallet::reward_address_from_address(&address)?;
                if raddr != rewardaddr {
                    return Err("ERROR stake address does not match one of the provided addresses, beware manipulation!".to_string().into());
                }
                rewardaddr = raddr
            }
        }
    }
    log::debug!("TxPattern okay!");
    Ok(())
}

pub fn create_response(
    bld_tx: &cardano::models::BuildOutput,
    raw_tx: &drasil_murin::utxomngr::RawTx,
    wallet_type: Option<&crate::datamodel::models::WalletType>,
) -> Result<crate::datamodel::models::UnsignedTransaction, drasil_murin::MurinError> {
    debug!("Try to store raw tx...");
    let tx_id = drasil_murin::utxomngr::txmind::store_raw_tx(raw_tx)?;
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
    nfts: &Vec<drasil_gungnir::minting::models::Nft>,
    policy_id: &String,
) -> Result<Vec<drasil_murin::MintTokenAsset>, drasil_murin::MurinError> {
    let mut out = Vec::<drasil_murin::MintTokenAsset>::new();
    for nft in nfts {
        out.push((
            Some(cardano::string_to_policy(policy_id)?),
            cardano::string_to_assetname(&nft.asset_name)?,
            cardano::u64_to_bignum(1),
        ))
    }
    Ok(out)
}
