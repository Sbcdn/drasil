/*
#################################################################################
# See LICENSE.md for full license information.                                  #
# Software: Drasil Blockchain Application Framework                             #
# License: Drasil Source Available License v1.0                                 #
# Licensors: Torben Poguntke (torben@drasil.io) & Zak Bassey (zak@drasil.io)    #
#################################################################################
*/
use cardano_serialization_lib as clib;
use cardano_serialization_lib::{tx_builder as ctxb, address as caddr, crypto as ccrypto, utils as cutils, plutus as plutus};
use clib::to_bytes;
use crate::hfn;
use crate::htypes;
use crate::error::MurinError;
use crate::marketplace::*;

/*
pub fn perform_buy( 
    fee         : &cutils::BigNum,
    sc_scripts  : &String, 
    sc_addr     : &String,
    gtxd        : &super::TxData,
    mptxd       : &super::marketplace::MpTxData,
    dummy       : bool
    ) -> Result<(clib::TransactionBody, clib::TransactionWitnessSet, clib::metadata::AuxiliaryData, TransactionUnspentOutputs, usize),MurinError> {
        
        if dummy == true {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Fee calcualtion------------------------------------------------");
            info!("---------------------------------------------------------------------------------------------------------\n"); 
        } else {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Build Transaction----------------------------------------------");
            info!("--------------------------------------------------------------------------------------------------------\n");
        }

         // Temp until Protocol Parameters fixed
        let mem = cutils::to_bignum(7000000u64);        //cutils::to_bignum(7000000u64);    
        let steps = cutils::to_bignum(2500000000u64);   //cutils::to_bignum(3000000000u64);
        let ex_unit_price : artitypes::ExUnitPrice = crate::ExUnitPrice { priceSteps: 7.21e-5, priceMemory:  5.77e-2 };
        //serde_json::from_str("{'executionUnitPrices': {'priceSteps': 7.21e-5,'priceMemory': 5.77e-2}").unwrap();
        let a = cutils::to_bignum(44u64);
        let b = cutils::to_bignum(155381u64);



info!("--------------------Iteration Ended------------------------------");
info!("Vkey Counter at End: {:?}",vkey_counter);
Ok((txbody,txwitness,aux_data,saved_input_txuos,vkey_counter))
}                


pub async fn build_mp_buy (gtxd : &super::TxData , mptxd : &super::MpTxData, sc_script: &String, sc_addr : &String) -> Result<artitypes::BuildOutput, MurinError> {

    // Temp until Protocol Parameters fixed
    let mem = cutils::to_bignum(7000000u64);        //cutils::to_bignum(7000000u64);    
    let steps = cutils::to_bignum(2500000000u64);   //cutils::to_bignum(3000000000u64);
    let ex_unit_price : artitypes::ExUnitPrice = crate::ExUnitPrice { priceSteps: 7.21e-5, priceMemory:  5.77e-2 };
    //serde_json::from_str("{'executionUnitPrices': {'priceSteps': 7.21e-5,'priceMemory': 5.77e-2}").unwrap();
    let a = cutils::to_bignum(44u64);
    let b = cutils::to_bignum(155381u64);

    //Create Tx
    let (txbody_,mut txwitness_, aux_data_, _,vkey_counter) = 
        perform_buy(&cutils::to_bignum(2000000), sc_script, sc_addr, gtxd, mptxd, true)?;

    let dummy_vkeywitnesses = artifn::make_dummy_vkeywitnesses(vkey_counter); 
    txwitness_.set_vkeys(&dummy_vkeywitnesses);

    // Build and encode dummy transaction
    let transaction_ = clib::Transaction::new(&txbody_,&txwitness_,Some(aux_data_.clone()));

    let calculated_fee = artifn::calc_txfee (&transaction_,&a,&b,ex_unit_price.clone(),&steps,&mem,true);   
    let (txbody, txwitness, aux_data,used_utxos,vkey_counter_2) = 
    perform_buy(&calculated_fee, sc_script, sc_addr, gtxd, mptxd, false)?;          

    let transaction2 = clib::Transaction::new(&txbody,&txwitness_,Some(aux_data.clone()));


    if vkey_counter_2 != vkey_counter || transaction2.to_bytes().len() != transaction_.to_bytes().len() {
        let dummy_vkeywitnesses = artifn::make_dummy_vkeywitnesses(vkey_counter_2); 
        txwitness_.set_vkeys(&dummy_vkeywitnesses);

        let calculated_fee = artifn::calc_txfee (&transaction2,&a,&b,ex_unit_price,&steps,&mem,true);   
        let (txbody, txwitness, aux_data,used_utxos,_) = 
            perform_buy(&calculated_fee, sc_script, sc_addr, gtxd, mptxd, false)?;     
        info!("Fee: {:?}",calculated_fee);
        debug!("\n\n\nDummy vkey_counter: {:?} \n\n",vkey_counter);
        debug!("\n\n\nDummy vkey_counter_2: {:?} \n\n",vkey_counter_2);
        Ok(artifn::tx_output_data (txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
    } else {
        info!("Fee: {:?}",calculated_fee);
        Ok(artifn::tx_output_data (txbody,txwitness,aux_data,used_utxos.to_hex()?,0u64,false)?)
    }            
}

*/