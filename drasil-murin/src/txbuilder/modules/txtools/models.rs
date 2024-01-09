use crate::clib;
use crate::clib::utils::BigNum;

pub type TokenAsset = (clib::PolicyID, clib::AssetName, BigNum);
pub type MintTokenAsset = (Option<clib::PolicyID>, clib::AssetName, BigNum);
