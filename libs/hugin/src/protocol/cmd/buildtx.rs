/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::{StdTxType, TransactionPattern};
use crate::protocol::stdtx;
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct BuildStdTx {
    customer_id: u64,
    txtype: StdTxType,
    txpattern: TransactionPattern,
}

impl BuildStdTx {
    pub fn new(customer_id: u64, txtype: StdTxType, txpattern: TransactionPattern) -> BuildStdTx {
        BuildStdTx {
            customer_id,
            txtype,
            txpattern,
        }
    }

    pub fn customer_id(&self) -> i64 {
        self.customer_id as i64
    }

    pub fn tx_type(&self) -> StdTxType {
        self.txtype.clone()
    }

    pub fn transaction_pattern(&self) -> TransactionPattern {
        self.txpattern.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<BuildStdTx> {
        let customer_id = parse.next_int()?;
        let txtype = parse.next_bytes()?;
        let txtype: StdTxType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txtype)?;
        let txpattern = parse.next_bytes()?;
        let txpattern: TransactionPattern = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txpattern)?;
        Ok(BuildStdTx {
            customer_id,
            txtype,
            txpattern,
        })
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response =
            Frame::Simple("ERROR: Could not build multisignature transaction".to_string());

        if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
            log::debug!("{:?}", response);
            response = Frame::Simple(e.to_string());
            dst.write_frame(&response).await?;
            return Err(Box::new(CmdError::InvalidData));
        }

        let ret = match self.tx_type() {
            StdTxType::DelegateStake => match stdtx::handle_stake_delegation(&self).await {
                Ok(s) => s,
                Err(e) => e.to_string(),
            },
        };

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

impl IntoFrame for BuildStdTx {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("stx".as_bytes()));

        frame.push_int(self.customer_id);

        let mtype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.txtype)
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
