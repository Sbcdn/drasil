use crate::datamodel::ContractType;
use crate::Parse;
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use drasil_murin::MurinError;

#[derive(Debug, Clone)]
pub struct FinalizeContract {
    customer_id: u64,
    ctype: ContractType,
    tx_id: String,
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
        let customer_id = parse
            .next_int()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

        let ctype = parse
            .next_bytes()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let ctype: ContractType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&ctype)?;

        let tx_id = parse
            .next_string()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let signature = parse
            .next_string()
            .map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;

        Ok(FinalizeContract {
            customer_id,
            ctype,
            tx_id,
            signature,
        })
    }

    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let raw_tx = drasil_murin::utxomngr::txmind::read_raw_tx(&self.get_tx_id())?;

        let response = match self.ctype {
            ContractType::MarketPlace => match self.finalize_marketplace(raw_tx).await {
                Ok(r) => r,
                Err(e) => Frame::Error(format!(
                    "Error: Finalizing Marketplace Transaction failed: {:?}",
                    e.to_string()
                )),
            },
            ContractType::WmEnRegistration => match self.finalize_general(raw_tx).await {
                Ok(r) => r,
                Err(e) => Frame::Error(format!(
                    "Error: Finalizing EarthNode Registration Transaction failed: {:?}",
                    e.to_string()
                )),
            },
            ContractType::WmtStaking => match self.finalize_general(raw_tx).await {
                Ok(r) => r,
                Err(e) => Frame::Error(format!(
                    "Error: Finalizing WmtStaking Transaction failed: {:?}",
                    e.to_string()
                )),
            },
            _ => Frame::Error(format!(
                "This contract Type does not exists {:?}",
                self.ctype
            )),
        };
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

    async fn finalize_general(&self, raw_tx: drasil_murin::RawTx) -> crate::Result<Frame> {
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
