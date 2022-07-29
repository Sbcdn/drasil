/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use crate::{Parse, CmdError};
use crate::{Connection,Frame,IntoFrame};
use crate::datamodel::{ContractType};

use bytes::Bytes;
use std::str::FromStr;
use tracing::{debug, instrument};
use bincode as bc;
use bc::Options;

#[derive(Debug,Clone)]
pub struct FinalizeContract {
    customer_id : u64,
    ctype       : ContractType,
    tx_id       : String,
    signature   : String, 
}

impl IntoFrame for FinalizeContract {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("fct".as_bytes()));

        frame.push_int(self.customer_id);
        
        let ctype_b = bc::DefaultOptions::new().with_varint_encoding().serialize(&self.ctype).unwrap();
        frame.push_bulk(Bytes::from(ctype_b));

        frame.push_bulk(Bytes::from(self.get_tx_id().into_bytes()));
        
        frame.push_bulk(Bytes::from(self.get_signature().into_bytes()));

        frame

    }
}

impl FinalizeContract {

    pub fn new(cid : u64, ctype: ContractType, tx_id : String, signature : String) -> FinalizeContract {
        FinalizeContract {
            customer_id : cid, 
            ctype : ctype,
            tx_id : tx_id,
            signature : signature,
        }
    }

    pub fn get_customer_id(&self) -> u64 {
        self.customer_id
    }

    pub fn get_contract_type(&self) -> ContractType {
        self.ctype.clone()
    }

    pub fn get_tx_id(&self) -> String {
        self.tx_id.clone()
    }

    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }


    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<FinalizeContract> {

        let customer_id = parse.next_int()?;

        let ctype = parse.next_bytes()?;
        let ctype : ContractType = bc::DefaultOptions::new().with_varint_encoding().deserialize(&ctype)?;
        
        let tx_id = parse.next_string()?;
        let tx_id = tx_id;

        let signature = parse.next_string()?;
        let signature = signature;

        Ok (
            FinalizeContract {
                customer_id : customer_id,
                ctype :ctype,
                tx_id : tx_id,
                signature : signature,
            }
        )
    }

    #[instrument(skip(self, dst))]
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
            
        let mut response = Frame::Simple("Error: something went wrong".to_string());
        let raw_tx = murin::utxomngr::txmind::read_raw_tx(&self.get_tx_id())?;

        if let Err(e) = murin::marketplace::MpTxData::from_str(raw_tx.get_tx_specific_rawdata()) {
            return Err(CmdError::Custom{str:format!("ERROR Invalid Transaction Data, this is not a marketplace transaction, {:?}",e.to_string())}.into());
        };
        
        match self.ctype {
            ContractType::MarketPlace => {
                response = self.finalize_marketplace(raw_tx).await?;
                
            },

            ContractType::NftShop => {

            },

            ContractType::NftMinter => {

            },

            ContractType::TokenMinter => {

            },

            _ => {
                return Err(CmdError::Custom{str:format!("This ccontract Type does not exists {:?}",self.ctype)}.into());
            }

        }
        
        // If successful:
        // ToDO:
        // store used Utxos into utxo manager and store txhash for ovserver
        //
        // store tx into permanent storage (drasildb)
        // delete tx from redis



        debug!(?response);
        dst.write_frame(&response).await?;
        Ok(())
    }

    async fn finalize_marketplace(&self, raw_tx: murin::RawTx) -> crate::Result<Frame> {
        use murin::txbuilders::finalize::finalize;
        let response = finalize(&self.get_signature(), raw_tx).await?;
        info!("Response: {}",response);
        Ok(Frame::Bulk(Bytes::from(bc::DefaultOptions::new().with_varint_encoding().serialize(&response)?)))
    }
}

