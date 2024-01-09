#![allow(non_snake_case)]
pub mod cip30;
pub mod models;
pub mod pparams;
pub mod supporting_functions;

pub use crate::MurinError;
use bech32::{self, ToBase32};
use cryptoxide::{blake2b::Blake2b, digest::Digest};
pub use models::*;
pub use supporting_functions::*;

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;

pub fn blake2b160(data: &[u8]) -> [u8; 20] {
    let mut out = [0u8; 20];
    let mut context = Blake2b::new(20);
    context.input(data);
    context.result(&mut out);
    Blake2b::blake2b(&mut out, data, &[]);
    out
}

pub fn string_to_policy(str: &String) -> Result<clib::PolicyID, MurinError> {
    Ok(clib::PolicyID::from_bytes(hex::decode(str)?)?)
}

pub fn string_to_assetname(str: &String) -> Result<clib::AssetName, MurinError> {
    Ok(clib::AssetName::new(hex::decode(str)?)?)
}

pub fn u64_to_bignum(n: u64) -> cutils::BigNum {
    cutils::to_bignum(n)
}

pub fn make_fingerprint(p: &String, a: &String) -> Result<String, MurinError> {
    let policy = hex::decode(p)?;
    let tn = hex::decode(a)?;
    let data = [&policy[..], &tn[..]].concat();
    let hash = blake2b160(&data);
    let fingerprint = bech32::Bech32::new("asset".to_string(), hash.to_base32()).unwrap();
    debug!("Maker Fingerprint: {:?}", fingerprint);
    Ok(fingerprint.to_string())
}
