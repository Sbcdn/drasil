use crate::datamodel::{MultiSigType, TransactionPattern};
use crate::protocol::multisig;
use crate::{CmdError, Parse};
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;

/// Command data for building a multisig transaction.
/// 
/// This can be used as a source of building blocks from which to assemble a multisig transaction.
#[derive(Debug, Clone)]
pub struct BuildMultiSig {
    customer_id: u64,
    /// The type of multisig transaction that the user wants to build
    mtype: MultiSigType,
    /// Specification of the basic attributes of this transaction (i.e. the aspects held in 
    /// common with all other transactions). This is chosen by the user in their HTTP request. 
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

    /// Find out what type of multisig transaction the user wants to build
    pub fn multisig_type(&self) -> MultiSigType {
        self.mtype.clone()
    }

    /// Get the specification of the basic attributes of this transaction (i.e. the aspects
    /// held in common with all other transactions)
    pub fn transaction_pattern(&self) -> TransactionPattern {
        self.txpattern.clone()
    }

    /// Parse the command parts (parts of a transaction request) into suitable types 
    /// and collect them into a single place in preparation for building a multisig 
    /// transaction. 
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

    /// Build a multisig transaction. `BuildMultiSig` (`self`) contains the building blocks used in this method.
    /// `dst` is the connection to the Heimdallr client (and thus indirectly to the user) who requested this transaction 
    /// to be built. This method sends a response back to this Heimdallr client (and thus back to the user who requested 
    /// this transaction to be built). 
    pub async fn apply(self, dst: &mut Connection) -> crate::Result<()> {
        let mut response =
            Frame::Simple("ERROR: Could not build multisignature transaction".to_string());
            
        // Make sure that the transaction pattern is valid for all multisig types except `ClAPIOneShotMint` and 
        // `CustomerPayout`, which won't get checked for valid transaction pattern. If transaction pattern is invalid,
        // the user receives error message in their HTTP response
        if self.multisig_type() != MultiSigType::ClAPIOneShotMint
            && self.multisig_type() != MultiSigType::CustomerPayout
        {
            if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
                log::debug!("{:?}", response);
                response = Frame::Simple(e.to_string());
                dst.write_frame(&response).await?;
                return Err(Box::new(CmdError::InvalidData));
            }
            log::debug!("Transaction pattern check okay!");
        }

        // Execute behavior/actions specific to the given multisig type
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
            MultiSigType::NftCollectionMinter => {
                log::debug!("NftCollectionMinter");
                ret = match multisig::handle_collection_mint(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            MultiSigType::ClAPIOneShotMint => {
                log::debug!("ClAPIOneShotMint");
                ret = match multisig::handle_onehshot_mint(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            /*  MultiSigType::TestRewards => {
                ret = match multisig::handle_testrewards(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            } */
            MultiSigType::CustomerPayout => {
                ret = match multisig::handle_customer_payout(&self).await {
                    Ok(s) => s,
                    Err(e) => e.to_string(),
                };
            }
            _ => {}
        }

        // Send HTTP response back to user who requested the multisig transaction to be built
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
