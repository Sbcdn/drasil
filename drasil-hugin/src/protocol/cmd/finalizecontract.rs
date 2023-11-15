use crate::datamodel::ContractType;
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use std::str::FromStr;

/// The parsed data attached to the incoming command that requests a smart-contract transaction to be finalized. 
#[derive(Debug, Clone)]
pub struct FinalizeContract {
    customer_id: u64,
    /// The type of smart contract that the user wants to finalize.
    ctype: ContractType,
    /// The specific built smart-contract transaction that the user wants to finalize.
    tx_id: String,
    /// Signature from the sender's private key to confirm that the owner of 
    /// the input UTxOs approves this smart-contract transaction
    signature: String,
}

impl IntoFrame for FinalizeContract {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("fct".as_bytes()));

        frame.push_int(self.customer_id);

        let ctype_b = bc::DefaultOptions::new()
            .with_varint_encoding()
            .serialize(&self.ctype)
            .unwrap();
        frame.push_bulk(Bytes::from(ctype_b));

        frame.push_bulk(Bytes::from(self.get_tx_id().into_bytes()));

        frame.push_bulk(Bytes::from(self.get_signature().into_bytes()));

        frame
    }
}

impl FinalizeContract {
    pub fn new(
        customer_id: u64,
        ctype: ContractType,
        tx_id: String,
        signature: String,
    ) -> FinalizeContract {
        FinalizeContract {
            customer_id,
            ctype,
            tx_id,
            signature,
        }
    }

    pub fn get_customer_id(&self) -> u64 {
        self.customer_id
    }

    /// Obtains the type of smart contract that the user wants to include in the transaction
    /// that the user wants to finalize
    pub fn get_contract_type(&self) -> ContractType {
        self.ctype.clone()
    }

    /// Obtains the specific built smart-contract transaction that the user wants to finalize.
    pub fn get_tx_id(&self) -> String {
        self.tx_id.clone()
    }

    /// Obtains the signature from the sender's private key that confirms that the owner of 
    /// the input UTxOs approves this transaction.
    pub fn get_signature(&self) -> String {
        self.signature.clone()
    }

    /// Parse the command parts (parts of a transaction request) into suitable types 
    /// and collect them into a single place in preparation for finalizing a smart-contract
    /// transaction. 
    pub(crate) fn parse_frames(parse: &mut Parse) -> crate::Result<FinalizeContract> {
        let customer_id = parse.next_int()?;

        let ctype = parse.next_bytes()?;
        let ctype: ContractType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&ctype)?;

        let tx_id = parse.next_string()?;
        let tx_id = tx_id;

        let signature = parse.next_string()?;
        let signature = signature;

        Ok(FinalizeContract {
            customer_id,
            ctype,
            tx_id,
            signature,
        })
    }

    /// Finalize a smart-contract transaction. `FinalizeContract` (`self`) contains the building blocks used in this method.
    /// `dst` is the connection to the Heimdallr client (and thus indirectly to the user) who requested the given transaction 
    /// to be finalized. This method sends a response back to this Heimdallr client (and thus back to the user who requested 
    /// the given transaction to be finalized). 
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response = Frame::Simple("Error: something went wrong".to_string());
        let raw_tx = drasil_murin::utxomngr::txmind::read_raw_tx(&self.get_tx_id())?;

        if let Err(e) =
            drasil_murin::marketplace::MpTxData::from_str(raw_tx.get_tx_specific_rawdata())
        {
            return Err(CmdError::Custom {
                str: format!(
                    "ERROR Invalid Transaction Data, this is not a marketplace transaction, {:?}",
                    e.to_string()
                ),
            }
            .into());
        };

        match self.ctype {
            ContractType::MarketPlace => {
                response = self.finalize_marketplace(raw_tx).await?;
            }

            ContractType::NftShop => {}

            ContractType::NftMinter => {}

            ContractType::TokenMinter => {}

            _ => {
                return Err(CmdError::Custom {
                    str: format!("This ccontract Type does not exists {:?}", self.ctype),
                }
                .into());
            }
        }

        // If successful:
        // ToDO:
        // store used Utxos into utxo manager and store txhash for ovserver
        //
        // store tx into permanent storage (drasildb)
        // delete tx from redis

        log::debug!("{:?}", response);
        dst.write_frame(&response).await?;
        Ok(())
    }

    async fn finalize_marketplace(&self, raw_tx: drasil_murin::RawTx) -> crate::Result<Frame> {
        use drasil_murin::txbuilder::finalize::finalize;
        let response = finalize(&self.get_signature(), raw_tx).await?;
        info!("Response: {}", response);
        Ok(Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&response)?,
        )))
    }
}
