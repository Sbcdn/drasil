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
use crate::datamodel::{StdTxType,TransactionPattern,ScriptSpecParams};

use bytes::Bytes;
use tracing::{debug, instrument};
use bincode as bc;
use bc::Options;


#[derive(Debug,Clone)]
pub struct BuildStdTx {
    customer_id : u64, 
    txtype : StdTxType,
    txpattern : TransactionPattern,
}

impl BuildStdTx {
    pub fn new(cid: u64, txtype: StdTxType, txpatter: TransactionPattern) -> BuildStdTx {
        BuildStdTx {
            customer_id : cid,
            txtype : txtype,
            txpattern : txpatter,
        }
    }

    pub fn customer_id(&self) -> u64 {
        self.customer_id
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
        let txtype : StdTxType = bc::DefaultOptions::new().with_varint_encoding().deserialize(&txtype)?;
        let txpattern = parse.next_bytes()?;
        let txpattern : TransactionPattern = bc::DefaultOptions::new().with_varint_encoding().deserialize(&txpattern)?;
        Ok (
            BuildStdTx {
                customer_id : customer_id,
                txtype :txtype,
                txpattern : txpattern,
            }
        )
    }

    #[instrument(skip(self, dst))]
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        dotenv::dotenv().ok();
        let mut response = Frame::Simple("ERROR: Could not build multisignature transaction".to_string());

        if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
            debug!(?response);
            response = Frame::Simple(e.to_string());
            dst.write_frame(&response).await?;
            ()        
        }

        let mut ret = String::new();
        match self.tx_type() {
            StdTxType::DelegateStake => {
                ret = match self.handle_stake_delegation().await {
                    Ok(s)  => s,
                    Err(e) => e.to_string(),
                }
            }
        }

        response = Frame::Bulk(Bytes::from(bc::DefaultOptions::new().with_varint_encoding().serialize(&ret)?));
        debug!(?response);
        dst.write_frame(&response).await?;

        Ok(())
    }

    async fn handle_stake_delegation(&self) -> crate::Result<String> { 

        match self.transaction_pattern().script().ok_or("ERROR: No specific contract data supplied")? {
            ScriptSpecParams::StakeDelegation{ 
                ..
            } =>   {
                ()
            },
            _ => {
                return Err(CmdError::Custom{str:format!("ERROR wrong data provided for '{:?}'",self.tx_type())}.into());
            }
            
        }

        let delegtxd = self.transaction_pattern().script().unwrap().into_stake_delegation().await?;
        let mut gtxd = self.transaction_pattern().into_txdata().await?;
        gtxd.set_user_id(self.customer_id);


        let dbsync = mimir::establish_connection()?;
        let slot = mimir::get_slot(&dbsync)?;
        gtxd.set_current_slot(slot as u64);

        let bech32_stake_addr = match gtxd.get_stake_address().to_bech32(None){
            Ok (ba) => ba,
            Err(e) => {
                return Err(CmdError::Custom{str:format!("Could not convert Stake Address;' {:?}'",e)}.into());
            }            
        };

        let registered = mimir::check_stakeaddr_registered(&bech32_stake_addr)?;

        let bld_tx = murin::delegation::build_delegation_tx(&gtxd, &delegtxd, &registered).await?;

        info!("Build Successful!");
        let tx = murin::utxomngr::RawTx::new(
            &bld_tx.get_tx_body(), 
            &bld_tx.get_txwitness(), 
            &bld_tx.get_tx_unsigned(),
            &bld_tx.get_metadata(),
            &gtxd.to_string(), 
            &delegtxd.to_string(),
            &bld_tx.get_used_utxos(),
            &hex::encode(gtxd.get_stake_address().to_bytes()),
            &(self.customer_id as i64),
            &(-1),
            &(-1.0),
        );
        debug!("RAWTX data: {:?}",tx);


        let ret = super::create_response(&bld_tx, &tx, self.transaction_pattern().wallet_type().as_ref())?;
        
        Ok(ret.to_string())
    }
    
}

impl IntoFrame for BuildStdTx {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("stx".as_bytes()));

        frame.push_int(self.customer_id);
        
        let mtype_b = bc::DefaultOptions::new().with_varint_encoding().serialize(&self.txtype).unwrap();
        frame.push_bulk(Bytes::from(mtype_b));

        let txpattern_b = bc::DefaultOptions::new().with_varint_encoding().serialize(&self.txpattern).unwrap();
        frame.push_bulk(Bytes::from(txpattern_b));

        frame

    }
}