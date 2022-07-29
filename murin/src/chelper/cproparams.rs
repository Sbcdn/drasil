/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use cardano_serialization_lib::{utils as cutils, fees};
use crate::htypes::*;


pub async fn get_protocol_parameter() {
    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    //
    //Set Protocol Parameter
    //
    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Read txFeePerByte from Protocol Parameters JSON

    /*
    let a = match  pp["txFeePerByte"].as_u64(){
        Some(num) => {cutils::to_bignum(num)}
        None      => {panic!("ERROR: Cant read txFeePerByte from ProtocolParameters")}
    };
    let b = match pp["txFeeFixed"].as_u64() {
        Some(num) => {cutils::to_bignum(num)}
        None      => {panic!("ERROR: Cant read txFeeFixed from ProtocolParameters");}
    };
    let min_utxo_val = match cutils::BigNum::from_str("1000000") {
        Ok(bignum) => {bignum}
        Err(err) => {panic!("Cant read bignum: {:?}",err);}
    };
    let pool_deposit = match pp["stakePoolDeposit"].as_u64() {
        Some(num) => {cutils::to_bignum(num)}
        None      => {panic!("ERROR: Cant read stakepool_deposit from ProtocolParameters");}
    };
    let key_deposit = match pp["stakeAddressDeposit"].as_u64() {
        Some(num) => {cutils::to_bignum(num)}
        None      => {panic!("ERROR: Cant read stakeAddressDeposit from ProtocolParameters");}
    };
    let coins_per_utxo_word = match pp["utxoCostPerWord"].as_u64() {
        Some(num) => {cutils::to_bignum(num)}
        None      => {panic!("ERROR: Cant read utxoCostPerWord from ProtocolParameters");}
    };
    let max_value_size : u32 = serde_json::from_value(pp["maxValueSize"].clone()).unwrap();
    let max_tx_size : u32  = serde_json::from_value(pp["maxTxSize"].clone()).unwrap(); 
    let linfee = fees::LinearFee::new(&a.clone(),&b.clone());

    let ex_unit_price : ExUnitPrice = serde_json::from_value(pp["executionUnitPrices"].clone()).unwrap();

    let mem = cutils::to_bignum(7000000u64);        //cutils::to_bignum(7000000u64);    
    let steps = cutils::to_bignum(2500000000u64);   //cutils::to_bignum(3000000000u64);
    */
}