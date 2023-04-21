/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use hugin::database::drasildb::*;
use hugin::drasildb::error::SystemDBError;

extern crate diesel;
use std::io::stdin;

fn main() -> Result<(), SystemDBError> {
    println!("Please provide user-id (i64):");
    let mut user_id = String::new();
    stdin().read_line(&mut user_id).unwrap();
    let user_id = &user_id[..(user_id.len() - 1)];
    let user_id = user_id.parse::<i64>()?;

    println!("Please provide contract-id (i64):");
    let mut contract_id = String::new();
    stdin().read_line(&mut contract_id).unwrap();
    let contract_id = &contract_id[..(contract_id.len() - 1)];
    let contract_id = contract_id.parse::<i64>()?;

    println!("Please provide contract-type (mp,nftshop,nftmint,tokmint):");
    let mut contract_type = String::new();
    stdin().read_line(&mut contract_type).unwrap();
    let contract_type = &contract_type[..(contract_type.len() - 1)];

    println!("Please provide a description (optional):");
    let mut description_ = String::new();
    stdin().read_line(&mut description_).unwrap();
    let description_ = &description_[..(description_.len() - 1)];
    let mut description: Option<&str> = None;
    if !description_.is_empty() {
        description = Some(description_)
    }

    println!("Please provide contract version (f32):");
    let mut version = String::new();
    stdin().read_line(&mut version).unwrap();
    let version = &version[..(version.len() - 1)];
    let version = version.parse::<f32>()?;

    println!("Please provide plutus script:");
    let mut plutus = String::new();
    stdin().read_line(&mut plutus).unwrap();
    let plutus = &plutus[..(plutus.len() - 1)];

    println!("Please provide script address script:");
    let mut address = String::new();
    stdin().read_line(&mut address).unwrap();
    let address = &address[..(address.len() - 1)];

    let contract = TBContracts::create_contract(
        &user_id,
        &contract_id,
        contract_type,
        description,
        &version,
        plutus,
        address,
        None,
        &false,
    )?;

    println!("Success! added: \n {contract:?}");
    Ok(())
}
