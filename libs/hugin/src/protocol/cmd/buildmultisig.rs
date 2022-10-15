/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::{MultiSigType, TransactionPattern};
use crate::protocol::multisig;
use crate::Parse;
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct BuildMultiSig {
    customer_id: u64,
    mtype: MultiSigType,
    txpattern: TransactionPattern,
}

impl BuildMultiSig {
    pub fn new(
        customer_id: u64,
        mtype: MultiSigType,
        txpattern: TransactionPattern,
    ) -> BuildMultiSig {
        BuildMultiSig {
            customer_id,
            mtype,
            txpattern,
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

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response =
            Frame::Simple("ERROR: Could not build multisignature transaction".to_string());
        if self.multisig_type() != MultiSigType::ClAPIOneShotMint {
            if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
                log::debug!("{:?}", response);
                response = Frame::Simple(e.to_string());
                dst.write_frame(&response).await?;
            }
            log::debug!("Transaction pattern check okay!");
        }

        let mut ret = String::new();
        match self.multisig_type() {
            MultiSigType::SpoRewardClaim => {
                ret = match multisig::handle_rewardclaim(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::NftVendor => {}
            MultiSigType::Mint => {
                ret = match multisig::handle_collection_mint(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::ClAPIOneShotMint => {
                ret = match multisig::handle_onehshot_mint(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::TestRewards => {
                ret = match multisig::handle_testrewards(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::CustomerPayout => {
                ret = match multisig::handle_customer_payout(&self).await {
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
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;

        Ok(())
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
