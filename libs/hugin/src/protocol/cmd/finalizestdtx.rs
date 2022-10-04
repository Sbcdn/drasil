/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::datamodel::StdTxType;
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct FinalizeStdTx {
    customer_id: u64,
    txtype: StdTxType,
    tx_id: String,
    signature: String,
}

impl IntoFrame for FinalizeStdTx {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("ftx".as_bytes()));

        frame.push_int(self.customer_id);

        let txtype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.txtype)
            .unwrap();
        frame.push_bulk(Bytes::from(txtype_b));

        frame.push_bulk(Bytes::from(self.get_tx_id().into_bytes()));

        frame.push_bulk(Bytes::from(self.get_signature().into_bytes()));

        frame
    }
}

impl FinalizeStdTx {
    pub fn new(cid: u64, txtype: StdTxType, tx_id: String, signature: String) -> FinalizeStdTx {
        FinalizeStdTx {
            customer_id: cid,
            txtype,
            tx_id,
            signature,
        }
    }

    pub fn get_customer_id(&self) -> u64 {
        self.customer_id
    }

    pub fn get_contract_type(&self) -> StdTxType {
        self.txtype.clone()
    }

    pub fn get_tx_id(&self) -> String {
        self.tx_id.clone()
    }

    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<FinalizeStdTx> {
        let customer_id = parse.next_int()?;
        let txtype = parse.next_bytes()?;
        let txtype: StdTxType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txtype)?;
        let tx_id = parse.next_string()?;
        let tx_id = tx_id;
        let signature = parse.next_string()?;
        let signature = signature;

        Ok(FinalizeStdTx {
            customer_id,
            txtype,
            tx_id,
            signature,
        })
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let raw_tx = murin::utxomngr::txmind::read_raw_tx(&self.get_tx_id())?;

        let used_utxos = raw_tx.get_usedutxos().clone();
        let ret = match self.txtype {
            StdTxType::DelegateStake => {
                if let Err(e) =
                    murin::delegation::DelegTxData::from_str(raw_tx.get_tx_specific_rawdata())
                {
                    return Err(CmdError::Custom{str:format!("ERROR Invalid Transaction Data, this is not a standard transaction, {:?}",e.to_string())}.into());
                };
                self.finalize_delegation(raw_tx.clone()).await?
            }
        };

        // store used Utxos into utxo manager and store txhash for ovserver
        murin::utxomngr::usedutxos::store_used_utxos(
            &ret,
            &murin::TransactionUnspentOutputs::from_hex(&used_utxos)?,
        )?;

        // ToDO:
        // store tx into permanent storage (drasildb)
        // delete build_tx from redis

        let response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret)?,
        ));
        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }

    async fn finalize_delegation(&self, raw_tx: murin::RawTx) -> crate::Result<String> {
        use murin::txbuilders::finalize::finalize;
        let response = finalize(&self.get_signature(), raw_tx).await?;
        info!("Response: {}", response);
        Ok(response)
    }
}
