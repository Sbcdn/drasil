use crate::datamodel::{StdTxType, TransactionPattern};
use crate::protocol::stdtx;
use crate::Parse; // CmdError
use crate::{Connection, Frame, IntoFrame};

use bc::Options;
use bincode as bc;
use bytes::Bytes;
use drasil_murin::MurinError;

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
        let customer_id = parse.next_int().map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let txtype = parse.next_bytes().map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
        let txtype: StdTxType = bc::DefaultOptions::new()
            .with_varint_encoding()
            .deserialize(&txtype)?;
        let txpattern = parse.next_bytes().map_err(|e| MurinError::ProtocolCommandError(e.to_string()))?;
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
        /*
        ToDo: Include Addresses Only Format to the check

        if let Err(e) = super::check_txpattern(&self.transaction_pattern()).await {
            log::debug!("{:?}", response);
            response = Frame::Simple(e.to_string());
            dst.write_frame(&response).await?;
            return Err(Box::new(CmdError::InvalidData));
        }
        */
        let ret = match self.tx_type() {
            StdTxType::DelegateStake => stdtx::handle_stake_delegation(&self).await
                .unwrap_or_else(|err| err.to_string()),
            StdTxType::DeregisterStake => stdtx::handle_stake_deregistration(&self).await
                .unwrap_or_else(|err| err.to_string()),
            StdTxType::StandardTx => stdtx::handle_stx(&self).await
                .unwrap_or_else(|err| err.to_string()),
            StdTxType::RewardWithdrawal => stdtx::handle_reward_withdrawal(&self).await
                .unwrap_or_else(|err| err.to_string()),
        };
        log::debug!("Return String before parsing into BC:\n{:?}", ret);
        let response = Frame::Bulk(Bytes::from(
            bc::DefaultOptions::new()
                .with_varint_encoding()
                .serialize(&ret)?,
        ));
        log::debug!("Response before writing into Frame{:?}", response);
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

#[cfg(test)]
mod tests {
    use crate::{TransactionPattern, Operation};

    #[test]
    fn new_build_std_tx_delegate_stake() {
        let customer_id = 0;
        let txtype = crate::StdTxType::DelegateStake;
        let poolhash = "pool1pt39c4va0aljcgn4jqru0jhtws9q5wj8u0xnajtkgk9g7lxlk2t".to_string();
        let addr1 = "addr_test1qp8cprhse9pnnv7f4l3n6pj0afq2hjm6f7r2205dz0583egaeu9dhacmtx94652q4ym0v9v2mcra0n28d5lrtjqzsgxqgk5t8s".to_string();
        let addresses = Some(vec![addr1]);
        let script_spec = Operation::StakeDelegation { poolhash, addresses };
        let network = 0;
        let txpattern = TransactionPattern::new_empty(customer_id, &script_spec, network);

        let build_std_tx = super::BuildStdTx::new(customer_id, txtype, txpattern);

        assert_eq!(
            build_std_tx.customer_id(),
            0
        );
        assert_eq!(
            build_std_tx.tx_type().to_string(),
            super::StdTxType::DelegateStake.to_string()
        );
        assert_eq!(
            build_std_tx.transaction_pattern().user(),
            "0"
        );
        assert_eq!(
            build_std_tx.transaction_pattern().contract_id(),
            None
        );
        assert_eq!(
            build_std_tx.transaction_pattern().wallet_type(),
            None
        );
        let trans_pat: Vec<&str> = vec![];
        assert_eq!(
            build_std_tx.transaction_pattern().used_addresses(),
            trans_pat
        );
        let utxos: Option<Vec<String>> = Some(vec![]);
        assert_eq!(
            build_std_tx.transaction_pattern().utxos(),
            utxos
        );
        assert_eq!(
            build_std_tx.transaction_pattern().excludes(),
            None
        );
        assert_eq!(
            build_std_tx.transaction_pattern().collateral(),
            None
        );
        assert_eq!(
            build_std_tx.transaction_pattern().network(),
            0
        );

    }

    // I couldn't find online resources on how Frames & Parse work.
    // #[test]
    // fn parse_build_std_tx_delegate_stake() {
    //     let frame1 = Frame::Simple("".to_string());
    //     let frame2 = Frame::Simple("".to_string());
    //     let arr_frame = Frame::Array(vec![frame1, frame2]);
    //     let mut parse = Parse::new(arr_frame).unwrap();
    //     let build_std_tx = super::BuildStdTx::parse_frames(&mut parse).unwrap();

    //     assert_eq!(
    //         build_std_tx.customer_id(),
    //         0
    //     );
    //     assert_eq!(
    //         build_std_tx.tx_type().to_string(),
    //         super::StdTxType::DelegateStake.to_string()
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().user(),
    //         "0"
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().contract_id(),
    //         None
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().wallet_type(),
    //         None
    //     );
    //     let trans_pat: Vec<&str> = vec![];
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().used_addresses(),
    //         trans_pat
    //     );
    //     let utxos: Option<Vec<String>> = Some(vec![]);
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().utxos(),
    //         utxos
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().excludes(),
    //         None
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().collateral(),
    //         None
    //     );
    //     assert_eq!(
    //         build_std_tx.transaction_pattern().network(),
    //         0
    //     );
    // }
}