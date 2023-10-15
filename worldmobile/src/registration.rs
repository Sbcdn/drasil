pub mod admin;
pub mod register;
pub mod unregister;

use murin::crypto::Ed25519KeyHash;
use murin::plutus::{PlutusData, PlutusDatumSchema};
use murin::utils::{to_bignum, BigNum};
use murin::AssetName;
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RegistrationDatum {
    pub validator_address: Vec<u8>,
    pub operator_address: Vec<u8>,
    pub moniker: Vec<u8>,
    #[serde(rename = "enUsedNftTn")]
    pub used_nft: AssetName,
    #[serde(rename = "enOwner")]
    pub owner: Ed25519KeyHash,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ElmConfig {
    pub validator_address: String,
    pub operator_address: String,
    pub moniker: String,
}

pub enum RegistrationRedeemer {
    Register,
    Unregister,
    Admin,
}

impl RegistrationRedeemer {
    pub fn redeemer(&self) -> BigNum {
        match &self {
            RegistrationRedeemer::Register => to_bignum(0),
            RegistrationRedeemer::Unregister => to_bignum(1),
            RegistrationRedeemer::Admin => to_bignum(2),
        }
    }
}

pub fn restore_wmreg_datum(bytes: &[u8]) -> Result<RegistrationDatum, Error> {
    let datum = PlutusData::from_bytes(bytes.to_vec()).expect("Could not deserialize PlutusData");
    tracing::debug!("Restored PlutusData: {:?}", datum);
    let d_str = datum
        .to_json(PlutusDatumSchema::DetailedSchema)
        .expect("Could not transform PlutusData to JSON");
    tracing::info!("Restored PlutusData Str: {:?}", d_str);
    let d_svalue = serde_json::from_str::<serde_json::Value>(&d_str)
        .expect("Could not transform PlutusDataJson to serde_json::Value");
    tracing::debug!("Deserialized Datum: \n{:?}", &d_str);
    let fields = d_svalue.get("fields").unwrap().as_array().unwrap();
    let operator_address = hex::decode(
        fields[0]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let validator_address = hex::decode(
        fields[1]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let moniker = hex::decode(
        fields[2]
            .as_object()
            .unwrap()
            .get("bytes")
            .unwrap()
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let used_nft = AssetName::new(
        hex::decode(
            fields[3]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    let owner = Ed25519KeyHash::from_bytes(
        hex::decode(
            fields[4]
                .as_object()
                .unwrap()
                .get("bytes")
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    let datum = RegistrationDatum {
        operator_address,
        validator_address,
        moniker,
        used_nft,
        owner,
    };
    Ok(datum)
}
