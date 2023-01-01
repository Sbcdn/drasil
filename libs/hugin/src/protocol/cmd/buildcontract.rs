/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::{
    ContractAction, ContractType, MarketplaceActions, ScriptSpecParams, TransactionPattern,
};
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use murin::MIN_ADA;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct BuildContract {
    customer_id: u64,
    ctype: ContractType,
    action: ContractAction,
    txpattern: TransactionPattern,
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

    pub fn customer_id(&self) -> u64 {
        self.customer_id
    }

    pub fn contract_type(&self) -> ContractType {
        self.ctype.clone()
    }

    pub fn action(&self) -> ContractAction {
        self.action.clone()
    }

    pub fn transaction_pattern(&self) -> TransactionPattern {
        self.txpattern.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<BuildContract> {
        let customer_id = parse.next_int()?;

        let ctype = parse.next_bytes()?;
        let ctype: ContractType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&ctype)?;

        let action = parse.next_string()?;
        let action = ContractAction::from_str(&action)?;

        let txpattern = parse.next_bytes()?;
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

        if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
            log::debug!("{:?}", response);
            response = Frame::Simple(e.to_string());
            dst.write_frame(&response).await?;
            return Err(Box::new(CmdError::InvalidData));
        }

        let mut ret = String::new();
        match self.ctype {
            ContractType::MarketPlace => {
                ret = match self.handle_marketplace().await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                }
            }

            ContractType::NftShop => {}

            ContractType::NftMinter => {}

            ContractType::TokenMinter => {}

            _ => {
                return Err(CmdError::Custom {
                    str: format!("ERROR his ccontract Type does not exists {:?}'", self.ctype),
                }
                .into());
            }
        }

        response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret.to_string())?,
        ));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }

    async fn handle_marketplace(&self) -> crate::Result<String> {
        match self
            .transaction_pattern()
            .script()
            .ok_or("ERROR: No specific contract data supplied")?
        {
            ScriptSpecParams::Marketplace {
                tokens,
                metadata,
                selling_price,
                ..
            } => {
                if tokens.is_empty()
                    || (metadata.is_empty()
                        && !(self.action()
                            == ContractAction::MarketplaceActions(MarketplaceActions::List)))
                    || (selling_price <= MIN_ADA * 3
                        && (self.action()
                            == ContractAction::MarketplaceActions(MarketplaceActions::List)
                            || self.action()
                                == ContractAction::MarketplaceActions(MarketplaceActions::Update)))
                {
                    return Err(CmdError::Custom {
                        str: format!(
                            "ERROR wrong data provided for script specific parameters: '{:?}'",
                            self.transaction_pattern().script()
                        ),
                    }
                    .into());
                }
            }
            _ => {
                return Err(CmdError::Custom {
                    str: format!("ERROR wrong data provided for '{:?}'", self.contract_type()),
                }
                .into());
            }
        }

        let mut gtxd = self.transaction_pattern().into_txdata().await?;
        let mptxd = self
            .transaction_pattern()
            .script()
            .unwrap()
            .into_mp(gtxd.clone().get_inputs())
            .await?;
        gtxd.set_user_id(self.customer_id as i64);
        let mut dbsync = mimir::establish_connection()?;
        let slot = mimir::get_slot(&mut dbsync)?;
        gtxd.set_current_slot(slot as u64);

        let ret: String;
        match self.action() {
            ContractAction::MarketplaceActions(mpa) => {
                match mpa {
                    MarketplaceActions::List => {
                        use crate::database::drasildb::*;
                        use murin::txbuilders::marketplace::list::*;
                        //build a listing and send the repsonse to the sender
                        let mut drasildbcon = establish_connection()?;
                        let contract = TBContracts::get_active_contract_for_user(
                            &mut drasildbcon,
                            self.customer_id as i64,
                            self.ctype.to_string(),
                            None,
                        )?;

                        let sc_addr = contract.address.to_string();
                        let sc_version = contract.version.to_string();

                        let mut dbsync = mimir::establish_connection()?;
                        let slot = mimir::get_slot(&mut dbsync)?;
                        gtxd.set_current_slot(slot as u64);

                        let res = build_mp_listing(&gtxd, &mptxd, &sc_addr, &sc_version).await?;

                        let tx = murin::utxomngr::RawTx::new(
                            &res.get_tx_body(),
                            &res.get_txwitness(),
                            &res.get_tx_unsigned(),
                            &res.get_metadata(),
                            &gtxd.to_string(),
                            &mptxd.to_string(),
                            &res.get_used_utxos(),
                            &hex::encode(gtxd.get_stake_address().to_bytes()),
                            &(self.customer_id as i64),
                            &[contract.contract_id],
                        );

                        ret = super::create_response(
                            &res,
                            &tx,
                            self.transaction_pattern().wallet_type().as_ref(),
                        )?
                        .to_string();
                    }
                    MarketplaceActions::Buy => {
                        ret = "Got MP Buy Transaction".to_string();
                    }
                    MarketplaceActions::Cancel => {
                        ret = "Got MP Cancel Transaction".to_string();
                    }
                    MarketplaceActions::Update => {
                        ret = "Got MP Update Transaction".to_string();
                    }
                }
            }
        }
        Ok(ret)
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
