use crate::clib;
use crate::clib::utils::BigNum;
//use crate::TransactionUnspentOutputs;

pub type TokenAsset = (clib::PolicyID, clib::AssetName, BigNum);
pub type MintTokenAsset = (Option<clib::PolicyID>, clib::AssetName, BigNum);

/*
pub(crate) type TxBO = (
    clib::TransactionBody,
    clib::TransactionWitnessSet,
    clib::metadata::AuxiliaryData,
    TransactionUnspentOutputs,
    usize,
);
 */
