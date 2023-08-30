#![allow(non_snake_case)]
pub mod supporting_functions;
pub mod models;

pub use super::MurinError;
use bech32::{self, ToBase32};
use cryptoxide::{blake2b::Blake2b, digest::Digest};
pub use supporting_functions::*;
pub use models::*;

use cardano_serialization_lib as clib;
use cardano_serialization_lib::utils as cutils;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

//use octavo_digest::Digest;
// use octavo_digest::blake2::*;

pub fn blake2b160(data: &[u8]) -> [u8; 20] {
    //Vec::<u8> {
    let mut out = [0u8; 20];
    let mut context = Blake2b::new(20);
    context.input(data);
    context.result(&mut out);
    Blake2b::blake2b(&mut out, data, &[]);
    out

    // let mut result = vec![0; 20];
    // let mut b2b = Blake2s160::default();

    // b2b.update(data);
    // b2b.result(&mut result);
    // println!("Result: {:?}",result);
    // result
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
    //policy.append(&mut tn.name());
    let hash = blake2b160(&data);
    let fingerprint = bech32::Bech32::new("asset".to_string(), hash.to_base32()).unwrap();
    debug!("Maker Fingerprint: {:?}", fingerprint);
    Ok(fingerprint.to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExUnitPrice {
    priceSteps: f64,
    priceMemory: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExUnit {
    steps: f64,
    memory: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProtocolVersion {
    minor: u32,
    major: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PlutusScriptV1 {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Cpp {
    maxValueSize: u64,
    minUTxOValue: Option<u64>,
    minPoolCost: u64,
    monetaryExpansion: f64,
    stakeAddressDeposit: u64,
    txFeeFixed: u64,
    poolRetireMaxEpoch: u32,
    stakePoolDeposit: u64,
    maxBlockExecutionUnits: ExUnit,
    stakePoolTargetNum: u32,
    maxBlockHeaderSize: u32,
    maxCollateralInputs: u32,
    txFeePerByte: u64,
    treasuryCut: f32,
    protocolVersion: ProtocolVersion,
    collateralPercentage: u32,
    poolPledgeInfluence: f32,
    maxTxExecutionUnits: ExUnit,
    executionUnitPrices: ExUnitPrice,
    decentralization: u32,
    utxoCostPerWord: u64,
    maxTxSize: u64,
    maxBlockBodySize: u64,
    //  costModels              : PlutusScriptV1,
}

impl Cpp {
    // ToDO:
    // Not happy with this solution, we would always read the parameters again,
    // it would be nice to store them for each start of hearth somehwere globaly in memory
    // and retrievem them from there without the need to read the file each time
    // Maybe via Redis ?

    pub fn get_protcol_parameters(path: Option<&String>) -> Result<Cpp, MurinError> {
        let path_pp: PathBuf = match path {
            None => {
                let path_pp_env = env::var("CARDANO_PROTOCOL_PARAMETER_PATH")?;
                PathBuf::from(path_pp_env)
            }
            Some(path) => PathBuf::from(path),
        };
        // Protocol Parameter JSON
        let pp_data = std::fs::read_to_string(path_pp)?;
        let pp: Cpp = serde_json::from_str(&pp_data)?;
        Ok(pp)
    }
}
