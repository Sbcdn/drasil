use std::str::FromStr;

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use drasil_murin::MurinError;

use crate::datamodel::{ContractAction, ContractType, TransactionPattern};
use crate::protocol::worldmobile::enreg::register;
use crate::protocol::worldmobile::staking;
use crate::protocol::worldmobile::staking::StakingAction;
use crate::Parse;
use crate::{Connection, Frame, IntoFrame};

#[derive(Debug, Clone)]
pub struct BuildContract {
    pub customer_id: u64,
    pub ctype: ContractType,
    pub action: ContractAction,
    pub txpattern: TransactionPattern,
}

impl BuildContract {
    pub fn new(
        customer_id: u64,
        ctype: ContractType,
        action: ContractAction,
        txpattern: TransactionPattern,
    ) -> BuildContract {
        BuildContract {
            customer_id,
            ctype,
            action,
            txpattern,
        }
    }

    pub fn customer_id(&self) -> i64 {
        self.customer_id as i64
    }

    pub fn contract_type(&self) -> &ContractType {
        &self.ctype
    }

    pub fn action(&self) -> &ContractAction {
        &self.action
    }

    pub fn transaction_pattern(&self) -> &TransactionPattern {
        &self.txpattern
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<BuildContract> {
        let customer_id = parse
            .next_int()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

        let ctype = parse
            .next_bytes()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let ctype: ContractType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&ctype)?;

        let action = parse
            .next_string()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let action = ContractAction::from_str(&action)?;

        let txpattern = parse
            .next_bytes()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let txpattern: TransactionPattern = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txpattern)?;

        Ok(BuildContract {
            customer_id,
            ctype,
            action,
            txpattern,
        })
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response = Frame::Simple("OK".to_string());

        if let Err(e) = super::check_txpattern(self.transaction_pattern()).await {
            log::debug!("{:?}", response);
            response = Frame::Simple(e.to_string());
            dst.write_frame(&response).await?;
            return Err(MurinError::ProtocolCommandErrorInvalidData);
        }

        let ret = match self.ctype {
            ContractType::MarketPlace => {
                crate::protocol::smartcontract::nft_marketplace::handle_marketplace(self)
                    .await
                    .unwrap_or_else(|err| err.to_string())
            }
            ContractType::WmtStaking => {
                let action = if let ContractAction::StakingAction(action) = self.action() {
                    action
                } else {
                    return Err(String::from("unexpected staking action").into());
                };
                match action {
                    StakingAction::Stake => staking::handle_wmt_stake(self)
                        .await
                        .unwrap_or_else(|err| err.to_string()),
                    StakingAction::UnStake => staking::handle_wmt_unstake(self)
                        .await
                        .unwrap_or_else(|err| err.to_string()),
                }
            }
            ContractType::WmEnRegistration => register::handle_en_registration(self)
                .await
                .unwrap_or_else(|err| err.to_string()),
            _ => {
                return Err(
                    format!("ERROR this contract Type does not exists {:?}'", self.ctype).into(),
                );
            }
        };

        response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret.to_string())?,
        ));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }
}

impl IntoFrame for BuildContract {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("bct".as_bytes()));

        frame.push_int(self.customer_id);

        let ctype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.ctype)
            .unwrap();
        frame.push_bulk(Bytes::from(ctype_b));

        frame.push_bulk(Bytes::from(self.action().to_string().into_bytes()));

        let txpattern_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.txpattern)
            .unwrap();
        frame.push_bulk(Bytes::from(txpattern_b));

        frame
    }
}
