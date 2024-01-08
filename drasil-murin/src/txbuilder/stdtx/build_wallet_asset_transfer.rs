use super::StandardTxData;
use crate::cardano::supporting_functions;
use crate::error::MurinError;
use crate::min_ada_for_utxo;
use crate::modules::transfer::models::{Sink, Source, TransBuilder, TransWallets, Transfer};
use crate::txbuilder::TxBO;
use crate::PerformTxb;
use crate::TxData;

use cardano_serialization_lib as clib;
use clib::address::Address;
use clib::metadata::{GeneralTransactionMetadata, MetadataMap, TransactionMetadatum};
use clib::utils::{hash_auxiliary_data, to_bignum};
use clib::{MultiAsset, TransactionOutput};

// One Shot Minter Builder Type
#[derive(Debug, Clone)]
pub struct AtSATBuilder {
    pub stxd: StandardTxData,
    pub fee_paying_address: Address,
    pub wallets: Option<TransWallets>,
}

impl AtSATBuilder {
    pub fn set_wallets(&mut self, wallets: TransWallets) {
        self.wallets = Some(wallets);
    }
    pub fn get_wallets(&self) -> Option<TransWallets> {
        self.wallets.clone()
    }
}

pub type AtSATParams<'a> = (&'a StandardTxData, &'a TransWallets, &'a Address);

impl<'a> PerformTxb<AtSATParams<'a>> for AtSATBuilder {
    fn new(t: AtSATParams) -> Self {
        AtSATBuilder {
            stxd: t.0.clone(),
            wallets: Some(t.1.clone()),
            fee_paying_address: t.2.clone(),
        }
    }

    fn perform_txb(
        &self,
        fee: &clib::utils::BigNum,
        gtxd: &TxData,
        _: &[String],
        fcrun: bool,
    ) -> std::result::Result<TxBO, MurinError> {
        if fcrun {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Fee Calculation------------------------------------------------");
            info!("---------------------------------------------------------------------------------------------------------\n");
        } else {
            info!("--------------------------------------------------------------------------------------------------------");
            info!("-----------------------------------------Build Transaction----------------------------------------------");
            info!("--------------------------------------------------------------------------------------------------------\n");
        }

        let mut builder = TransBuilder::new(&self.fee_paying_address);
        let wallets = if let Some(mut w) = self.wallets.to_owned() {
            w.wallets.iter_mut().for_each(|n| {
                if let Ok(Some(used_utxos)) =
                    crate::utxomngr::usedutxos::check_any_utxo_used(&n.utxos)
                {
                    n.utxos.remove_used_utxos(used_utxos);
                }
            });
            log::debug!("\n\nTook Input wallets");
            w
        } else {
            log::debug!("\n\nCreated empty wallet in wallet asset transfer");
            TransWallets::new()
        };
        if let Some(w) = self.get_wallets() {
            builder.wallets = w;
        }

        /////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Auxiliary Data
        //  Plutus Script and Metadata
        /////////////////////////////////////////////////////////////////////////////////////////////////////

        let mut aux_data = clib::metadata::AuxiliaryData::new();
        let mut option_aux_data = None;
        let mut gtm = GeneralTransactionMetadata::new();

        let mut map = MetadataMap::new();

        for (i, t) in self.stxd.transfers.iter().enumerate() {
            if let Some(m) = &t.metadata {
                if m.len() > 100 {
                    return Err(MurinError::new("Message must have max 100 characters."));
                }
                let mut byte_string_array: Vec<String> = vec![];
                let single_byte_string = m.clone().into_bytes();
                if single_byte_string.len() > 64 {
                    single_byte_string.chunks(64).for_each(|a| {
                        let component_byte_string = String::from_utf8(a.to_vec()).unwrap();
                        byte_string_array.push(component_byte_string);
                    });
                } else {
                    byte_string_array.push(String::from_utf8(single_byte_string.to_vec()).unwrap());
                }

                map.insert(
                    &TransactionMetadatum::new_int(&clib::utils::Int::new(&to_bignum(i as u64))),
                    &clib::metadata::TransactionMetadatum::new_text(m.to_string()).unwrap(),
                );
            }
        }

        let aux_data_hash = if map.len() > 0 {
            let data = TransactionMetadatum::new_map(&map);
            gtm.insert(&clib::utils::BigNum::from_str("0").unwrap(), &data);
            aux_data.set_metadata(&gtm);
            let auxhash = hash_auxiliary_data(&aux_data);
            option_aux_data = Some(aux_data);
            Some(auxhash)
        } else {
            None
        };

        //////////////////////////////////////////////////////////////////////////////////////////////////////
        //Add Inputs and Outputs
        //
        //
        ///////////////////////////////////////////////////////////////////////////////////////////////////////
        let mut transfers = Vec::<Transfer>::new();
        for trans in &self.stxd.transfers {
            let mut value = trans
                .assets
                .iter()
                .fold(clib::utils::Value::zero(), |mut acc, a| {
                    match &a.policy {
                        // Ada
                        None => {
                            if a.tokenname.is_some() {
                                clib::utils::Value::zero()
                            } else {
                                acc = acc
                                    .checked_add(&clib::utils::Value::new(&a.amount))
                                    .unwrap();
                                acc
                            }
                        }
                        // Other
                        Some(p) => {
                            if a.tokenname.is_none() {
                                clib::utils::Value::zero()
                            } else {
                                let mut assets = clib::Assets::new();
                                assets.insert(a.tokenname.as_ref().unwrap(), &a.amount);
                                let mut ma = MultiAsset::new();
                                ma.insert(p, &assets);
                                acc = acc
                                    .checked_add(&clib::utils::Value::new_with_assets(
                                        &to_bignum(0),
                                        &ma,
                                    ))
                                    .unwrap();
                                acc
                            }
                        }
                    }
                });
            let min_utxo_val =
                min_ada_for_utxo(&TransactionOutput::new(&self.fee_paying_address, &value))?
                    .amount();

            if value.coin().compare(&min_utxo_val.coin()) == -1 {
                value.set_coin(&min_utxo_val.coin())
            }

            let sink = Sink::new(&trans.receiver, &value);
            let mut source = Source::new(&self.fee_paying_address);
            source.set_pay_value(value);
            let trans = Transfer::new(&source, &vec![sink]);
            transfers.push(trans);
        }

        builder.transfers = transfers;
        builder.wallets = wallets;
        builder.build(*fee)?;

        let saved_input_txuos = builder.tx.clone().unwrap().0;
        let vkey_counter =
            supporting_functions::get_vkey_count(&builder.tx.as_ref().unwrap().0, None);
        let slot = to_bignum(
            gtxd.clone().get_current_slot()
                + supporting_functions::get_ttl_tx(&gtxd.clone().get_network()),
        );
        let mut txbody = clib::TransactionBody::new_tx_body(
            &builder.tx.as_ref().unwrap().1,
            &builder.tx.as_ref().unwrap().2,
            fee,
        );
        txbody.set_ttl(&slot);

        if let Some(aux_data_hash) = aux_data_hash {
            txbody.set_auxiliary_data_hash(&aux_data_hash);
        }

        let txwitness = clib::TransactionWitnessSet::new();

        debug!("TxWitness: {:?}", hex::encode(txwitness.to_bytes()));
        debug!("TxBody: {:?}", hex::encode(txbody.to_bytes()));
        debug!("--------------------Iteration Ended------------------------------");
        debug!("Vkey Counter at End: {:?}", vkey_counter);
        Ok((
            txbody,
            txwitness,
            option_aux_data,
            saved_input_txuos,
            vkey_counter,
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::env::set_var;

    use crate::cardano::TransactionUnspentOutputs;
    use crate::clib::address::Address;
    use crate::modules::transfer::models::TransWallet;
    use crate::stdtx::{build_wallet_asset_transfer::AtSATBuilder, StandardTxData};
    use crate::txbuilder::modules::transfer::models::TransWallets;
    use crate::wallet::reward_address_from_address;
    use crate::PerformTxb;

    use crate::MurinError;

    #[tokio::test]
    async fn test_metadata_for_stdtx_1transfer() -> Result<(), MurinError> {
        let pp = "{
            \"collateralPercentage\": 150,
            \"costModels\": {
                \"PlutusScriptV1\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10},
                \"PlutusScriptV2\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"serialiseData-cpu-arguments-intercept\":1159724,\"serialiseData-cpu-arguments-slope\":392670,\"serialiseData-memory-arguments-intercept\":0,\"serialiseData-memory-arguments-slope\":2,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEcdsaSecp256k1Signature-cpu-arguments\":20000000000,\"verifyEcdsaSecp256k1Signature-memory-arguments\":20000000000,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10,\"verifySchnorrSecp256k1Signature-cpu-arguments-intercept\":20000000000,\"verifySchnorrSecp256k1Signature-cpu-arguments-slope\":0,\"verifySchnorrSecp256k1Signature-memory-arguments\":20000000000}
            },
            \"decentralization\": null,
            \"executionUnitPrices\": {
                \"priceMemory\": 5.77e-2,
                \"priceSteps\": 7.21e-5
            },
            \"extraPraosEntropy\": null,
            \"maxBlockBodySize\": 90112,
            \"maxBlockExecutionUnits\": {
                \"memory\": 62000000,
                \"steps\": 40000000000
            },
            \"maxBlockHeaderSize\": 1100,
            \"maxCollateralInputs\": 3,
            \"maxTxExecutionUnits\": {
                \"memory\": 14000000,
                \"steps\": 10000000000
            },
            \"maxTxSize\": 16384,
            \"maxValueSize\": 5000,
            \"minPoolCost\": 340000000,
            \"minUTxOValue\": null,
            \"monetaryExpansion\": 3.0e-3,
            \"poolPledgeInfluence\": 0.3,
            \"poolRetireMaxEpoch\": 18,
            \"protocolVersion\": {
                \"major\": 7,
                \"minor\": 0
            },
            \"stakeAddressDeposit\": 2000000,
            \"stakePoolDeposit\": 500000000,
            \"stakePoolTargetNum\": 500,
            \"treasuryCut\": 0.2,
            \"txFeeFixed\": 155381,
            \"txFeePerByte\": 44,
            \"utxoCostPerByte\": 4310,
            \"utxoCostPerWord\": null
        }";
        std::fs::write("protocol_parameters_babbage_test.json", pp).unwrap();
        set_var(
            "CARDANO_PROTOCOL_PARAMETER_PATH",
            "protocol_parameters_babbage_test.json",
        );
        let std_asset_txd = serde_json::from_str::<StandardTxData>("{
            \"wallet_addresses\": [
                \"addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2\",
                \"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"addr_test1qqdp3cry5vc2gfjljctdu638tvkcqfx40fjunht9hrmru5zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qnaxxgs\",
                \"addr_test1qr2mw080ujz0unmpn9lx5ftfuewc6htyr6v3a0svul2zgezcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qgryf7t\",
                \"addr_test1qr7tqh7tsg4lut3jv6tsfwlv464m6knjjw90ugyz8uzgr6zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qt0jxzj\",
                \"addr_test1qrscurjp292sxv24sepj7ghq4ydkkekzaz53zwfswcna6ljcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6q8pu3l5\",
                \"addr_test1qqssrphse6qmp9h0ksu5vfmsx99tfl2lc6rhvy2spd5wr86cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw59j4j\",
                \"addr_test1qqgagc0fy6nm0qe4h8zqxsg952tqjeg7l7j0agd0cx4u25zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qxvept2\",
                \"addr_test1qrjmru0chcxw0q7y099k2elzr45sh77gafkzj75xqwe66zzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qthp2py\",
                \"addr_test1qq78tygxpu7a53rz7m6jnrtf5s8sc6dvg63jz80uyqrfswzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qhe9ekw\",
                \"addr_test1qpvntdhn6s9d09z72f75atv8ha8qax46a5tfpcf7cp2jwm6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6ql87t33\",
                \"addr_test1qqrja5l2hdl5gdyz7xvm948jg7vc9ed0uzp28yqgveaxww6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qkw5xzz\",
                \"addr_test1qpzmsl9qfyzlh94049ya2ffjy8akvhmrhc6azdccmdyn2j2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qzudgs9\"
            ],
            \"transfers\":[
            {
                \"receiver\":\"addr_test1vz34ylm8ucm0xgq0a72n0r3w7yhgdudxxekvsae5j3w5d5sje670h\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you\"
            }
        ]
        }")?;

        let mut wallets = TransWallets::new();

        // Input is from DBsync so we have live data, better would be a fixed data set
        let inputs = TransactionUnspentOutputs::from_hex("9828588e828258203182c2a0a4d98cf4fe8e491cbf9068e43b100842eec7cbe3319b9f4b16e8fa820082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582044b6f488071f1e105f709a2f0df98b465331b73e8374be56f33af703589ccc0f0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588f8282582045c6b0ddcf6b58d91ae99671fc8efca89abbb60826871db3eb33c97f9684bff80082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004949b3a1581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a19096d588e8282582033ba220a75ca5734bdc041f0f6005f69db0cce225efa98af1cd74f747873a69b0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901078282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a22df7a93a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a146744452415341190af0581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea1457454657374192b39581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a190122581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b190105588e82825820f0de9976070444386a4070434cc18f21c6dd0cd3c53b49050c3dcec84d6535570082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258209907fcf5847e61f3724aa2c3165b42eb37912620bd6e3da6f119fae4f3418e710082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820aeaab7142e18485b35b473122499913eafa3613b353cdb5a013f1875ce3884d50082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c29470125cc1663361a1867494a1c9f39a4ae267b1aa55f7ff87ef0468aeee4c0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208ae4fa04190022b870725f075eadd01c55ffa3a4d67f21356ae0310ded03f81f0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004c4b40a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b18c8588e82825820f67ac37cf0a91615fc2d8415835921650a9217c36df62e1321d1271519d2194500825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ea6ebb93e208716cb3cb0cac47066f2a47a4e472274483fdc18901228596754900825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e037166b40a9fbcf51003ddee74c7f71d25f0e30e55992a12bc792692f2585e100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582005e27d72124d535a80439710c77cb55cf406962b0a92c6c928f9f635b0fa46d700825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015899828258200ca67284834d0714809151fc8b8e8c7747e119786e5a734949f46391b024122400825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a001226b8a1581c868f75968b696aa86be159ea63f31221df4cd4b7a48159fb632968b3a24774656e4e465433014774656e4e46543501588e8282582075710b4252f599a97f9d05e4baafbd2d78fffc79be3879c94ef77ba8626eba1100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820b9c3ac99ffcb6eaa806609bbb9350d7369be837f1a58972ebec6225f935d5b2e0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e8d6059bd8627cd69f31445bc03d0c5221d9130fed7e47cea674e61c29302c8a0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820f0d8fe5ab94aacd7901cdbca325b5003d4767bef5093441abbcdfe3e1f345c1b0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206be7d565e9844c813f71e76fa0daa99be67d5d558eeb7d6cb5483f7cb843b1310082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ff79939f8235e2b176b5673fb0ecbdbf3299870f80ac075248ceff8bdeeabc040082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206ef58e3560d3e490eef8da9e32bc0e2ae6cf7f8f213e3ae3d97ebe795be4989d0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820940e7bdb43b3e2242918e79afe478ba7d45a6467cf8388d90de36b17f8834a2b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582054f16809c7c94af95af7550c5d720d0342474b2da1a73a753f690727f633be8c0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258200a4784e70f6d6c05527926f2ca5a0b698429cfaf445889d01e5c4af344c65de50082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820a344a87c16823d5bb509699fa3bc14d4a2532ad4236d11763e5022476020233b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208d7d59e3e5705a5b4c5cf0e3dc50ffb2cf30c7c2ce30fb2e232459bf5c755d0a0082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258201b14f36df5f17fdb543246d2ad019431473e6fdd39be1599b52f68605d0fd0c40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e178ea9c00d9009476b321e6de55aca918c2130c965475471cd45c937f64fff40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258207e0fd50e46b874e9754d520c4097b93ec9af7e152b19b032567ee30bd555f4970082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f53490158948282582099c66e979975ec4ae94487bcfca40372738d0879b79e2d6a9beba41deabff0e50082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0011d28aa1581c0e11a695e4fd6e28dbe13609e59989c2e3fd73b8d17dcd6638ded4dca14b64697363324e465431303001588e82825820fa4ed66b88147b1ad7d8774f67490ff22e3157fc5babbd03c5e947891d5087630082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206a41da9bdb24cac56af60432fe5b9418a8aede9623e2997ce7a469032916899b0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258204dde9003d57183d23ff19cfd4275dc8136e695703c9d74d9015d8d634bc6a02d0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582046f8f9a4a5e39df190d9083bdf6f3e4383cd49fb60ed6ecc546f01eed7b2078c0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c25bb18afe488cff815644a91e283e870c2d12212b484a8b8038a1db237f12d80082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c22795d4c0a4c423451cc9d669640daab6d9b5e0aaafaa54142a343dd66e59330082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901c482825820091c4a9d61ce3b94a4b0e6d1800231250a2b0a7401b55eeff231a6fabf0732c3008258390011d461e926a7b78335b9c4034105a29609651effa4fea1afc1abc55058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0038d318a1581c4c5ac6739376849c917d299a4ef3c74b44cfb1a0ebd4948877058559b8194a45617274684e6f646531014a45617274684e6f646532014a45617274684e6f646533014a45617274684e6f646534014a45617274684e6f646535014a45617274684e6f646536014a45617274684e6f646537014a45617274684e6f646538014a45617274684e6f646539014b45617274684e6f64653130014b45617274684e6f64653131014b45617274684e6f64653132014b45617274684e6f64653133014b45617274684e6f64653134014b45617274684e6f64653135014b45617274684e6f64653136014b45617274684e6f64653137014b45617274684e6f64653138014b45617274684e6f64653139014b45617274684e6f64653230014b45617274684e6f64653231014b45617274684e6f64653232014b45617274684e6f64653233014b45617274684e6f64653234014b45617274684e6f6465323501588d828258201b07f1152e52ce0a9dbb561aa2e2d1750ca3a1a4141150a8bad342947a66a3a60182583900e5b1f1f8be0ce783c4794b6567e21d690bfbc8ea6c297a8603b3ad0858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00155cc0a1581cc2f5ddefa7f7e091f202828bf3692ac5c39833068aacf5cdfebbebdaa1444e46543201")?;
        let network = crate::NetworkIdKind::Testnet;
        let addr = Address::from_bech32("addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2")?;
        let gtxd: crate::TxData = crate::TxData::new(
            None,
            vec![addr.clone()],
            Some(reward_address_from_address(&addr).unwrap()),
            inputs,
            network,
            120000,
        )?;

        let uw = TransWallet::new(&addr, &gtxd.get_inputs());
        wallets.add_wallet(&uw);

        // build tx
        let txb_param: (&StandardTxData, &TransWallets, &Address) =
            (&std_asset_txd, &wallets, &addr);
        let standard_tx_builder = AtSATBuilder::new(txb_param);
        let txbuilder = crate::TxBuilder::new(&gtxd, &vec![]);
        let bld_tx = txbuilder.build(&standard_tx_builder).await.unwrap();
        let tx_org = crate::clib::Transaction::new(
            &bld_tx.get_tx_body_typed(),
            &bld_tx.get_txwitness_typed(),
            Some(bld_tx.get_metadata_typed()),
        );
        // println!(
        //     "\nOriginal CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );

        let tx_restored: crate::Transaction =
            crate::clib::Transaction::from_bytes(hex::decode("84a5008182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e192701018282581d60a3527f67e636f3200fef95378e2ef12e86f1a6366cc87734945d46d2821a004c4b40a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b18c882583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a229074c2a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a146744452415341190af0581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea1457454657374192b39581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a190122581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b183d021a0002ba91031a0001dbc8075820013e5545b36d84b420f479874abc385de130814b5c5a75614bc27a1a74c1aca4a0f5a100a100781f48656c6c6f204d7920667269656e64207468697320697320666f7220796f75")?)?;

        // println!(
        //     "\nRestored CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );

        assert_eq!(
            hex::encode(tx_org.to_bytes()),
            hex::encode(tx_restored.to_bytes())
        );
        std::fs::remove_file(std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH").unwrap()).unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_metadata_for_stdtx_2transfers() -> Result<(), MurinError> {
        let pp = "{
            \"collateralPercentage\": 150,
            \"costModels\": {
                \"PlutusScriptV1\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10},
                \"PlutusScriptV2\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"serialiseData-cpu-arguments-intercept\":1159724,\"serialiseData-cpu-arguments-slope\":392670,\"serialiseData-memory-arguments-intercept\":0,\"serialiseData-memory-arguments-slope\":2,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEcdsaSecp256k1Signature-cpu-arguments\":20000000000,\"verifyEcdsaSecp256k1Signature-memory-arguments\":20000000000,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10,\"verifySchnorrSecp256k1Signature-cpu-arguments-intercept\":20000000000,\"verifySchnorrSecp256k1Signature-cpu-arguments-slope\":0,\"verifySchnorrSecp256k1Signature-memory-arguments\":20000000000}
            },
            \"decentralization\": null,
            \"executionUnitPrices\": {
                \"priceMemory\": 5.77e-2,
                \"priceSteps\": 7.21e-5
            },
            \"extraPraosEntropy\": null,
            \"maxBlockBodySize\": 90112,
            \"maxBlockExecutionUnits\": {
                \"memory\": 62000000,
                \"steps\": 40000000000
            },
            \"maxBlockHeaderSize\": 1100,
            \"maxCollateralInputs\": 3,
            \"maxTxExecutionUnits\": {
                \"memory\": 14000000,
                \"steps\": 10000000000
            },
            \"maxTxSize\": 16384,
            \"maxValueSize\": 5000,
            \"minPoolCost\": 340000000,
            \"minUTxOValue\": null,
            \"monetaryExpansion\": 3.0e-3,
            \"poolPledgeInfluence\": 0.3,
            \"poolRetireMaxEpoch\": 18,
            \"protocolVersion\": {
                \"major\": 7,
                \"minor\": 0
            },
            \"stakeAddressDeposit\": 2000000,
            \"stakePoolDeposit\": 500000000,
            \"stakePoolTargetNum\": 500,
            \"treasuryCut\": 0.2,
            \"txFeeFixed\": 155381,
            \"txFeePerByte\": 44,
            \"utxoCostPerByte\": 4310,
            \"utxoCostPerWord\": null
        }";
        std::fs::write("protocol_parameters_babbage_test.json", pp).unwrap();
        set_var(
            "CARDANO_PROTOCOL_PARAMETER_PATH",
            "protocol_parameters_babbage_test.json",
        );
        let std_asset_txd = serde_json::from_str::<StandardTxData>("{
            \"wallet_addresses\": [
                \"addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2\",
                \"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"addr_test1qqdp3cry5vc2gfjljctdu638tvkcqfx40fjunht9hrmru5zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qnaxxgs\",
                \"addr_test1qr2mw080ujz0unmpn9lx5ftfuewc6htyr6v3a0svul2zgezcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qgryf7t\",
                \"addr_test1qr7tqh7tsg4lut3jv6tsfwlv464m6knjjw90ugyz8uzgr6zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qt0jxzj\",
                \"addr_test1qrscurjp292sxv24sepj7ghq4ydkkekzaz53zwfswcna6ljcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6q8pu3l5\",
                \"addr_test1qqssrphse6qmp9h0ksu5vfmsx99tfl2lc6rhvy2spd5wr86cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw59j4j\",
                \"addr_test1qqgagc0fy6nm0qe4h8zqxsg952tqjeg7l7j0agd0cx4u25zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qxvept2\",
                \"addr_test1qrjmru0chcxw0q7y099k2elzr45sh77gafkzj75xqwe66zzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qthp2py\",
                \"addr_test1qq78tygxpu7a53rz7m6jnrtf5s8sc6dvg63jz80uyqrfswzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qhe9ekw\",
                \"addr_test1qpvntdhn6s9d09z72f75atv8ha8qax46a5tfpcf7cp2jwm6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6ql87t33\",
                \"addr_test1qqrja5l2hdl5gdyz7xvm948jg7vc9ed0uzp28yqgveaxww6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qkw5xzz\",
                \"addr_test1qpzmsl9qfyzlh94049ya2ffjy8akvhmrhc6azdccmdyn2j2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qzudgs9\"
            ],
            \"transfers\":[
            {
                \"receiver\":\"addr_test1vz34ylm8ucm0xgq0a72n0r3w7yhgdudxxekvsae5j3w5d5sje670h\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you\"
            },
            {
                \"receiver\":\"addr_test1vz34ylm8ucm0xgq0a72n0r3w7yhgdudxxekvsae5j3w5d5sje670h\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 2\"
            }
        ]
        }")?;

        let mut wallets = TransWallets::new();

        // Input is from DBsync so we have live data, better would be a fixed data set
        let inputs = TransactionUnspentOutputs::from_hex("9828588e828258203182c2a0a4d98cf4fe8e491cbf9068e43b100842eec7cbe3319b9f4b16e8fa820082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582044b6f488071f1e105f709a2f0df98b465331b73e8374be56f33af703589ccc0f0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588f8282582045c6b0ddcf6b58d91ae99671fc8efca89abbb60826871db3eb33c97f9684bff80082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004949b3a1581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a19096d588e8282582033ba220a75ca5734bdc041f0f6005f69db0cce225efa98af1cd74f747873a69b0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901078282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a22df7a93a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a146744452415341190af0581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea1457454657374192b39581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a190122581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b190105588e82825820f0de9976070444386a4070434cc18f21c6dd0cd3c53b49050c3dcec84d6535570082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258209907fcf5847e61f3724aa2c3165b42eb37912620bd6e3da6f119fae4f3418e710082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820aeaab7142e18485b35b473122499913eafa3613b353cdb5a013f1875ce3884d50082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c29470125cc1663361a1867494a1c9f39a4ae267b1aa55f7ff87ef0468aeee4c0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208ae4fa04190022b870725f075eadd01c55ffa3a4d67f21356ae0310ded03f81f0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004c4b40a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b18c8588e82825820f67ac37cf0a91615fc2d8415835921650a9217c36df62e1321d1271519d2194500825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ea6ebb93e208716cb3cb0cac47066f2a47a4e472274483fdc18901228596754900825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e037166b40a9fbcf51003ddee74c7f71d25f0e30e55992a12bc792692f2585e100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582005e27d72124d535a80439710c77cb55cf406962b0a92c6c928f9f635b0fa46d700825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015899828258200ca67284834d0714809151fc8b8e8c7747e119786e5a734949f46391b024122400825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a001226b8a1581c868f75968b696aa86be159ea63f31221df4cd4b7a48159fb632968b3a24774656e4e465433014774656e4e46543501588e8282582075710b4252f599a97f9d05e4baafbd2d78fffc79be3879c94ef77ba8626eba1100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820b9c3ac99ffcb6eaa806609bbb9350d7369be837f1a58972ebec6225f935d5b2e0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e8d6059bd8627cd69f31445bc03d0c5221d9130fed7e47cea674e61c29302c8a0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820f0d8fe5ab94aacd7901cdbca325b5003d4767bef5093441abbcdfe3e1f345c1b0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206be7d565e9844c813f71e76fa0daa99be67d5d558eeb7d6cb5483f7cb843b1310082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ff79939f8235e2b176b5673fb0ecbdbf3299870f80ac075248ceff8bdeeabc040082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206ef58e3560d3e490eef8da9e32bc0e2ae6cf7f8f213e3ae3d97ebe795be4989d0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820940e7bdb43b3e2242918e79afe478ba7d45a6467cf8388d90de36b17f8834a2b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582054f16809c7c94af95af7550c5d720d0342474b2da1a73a753f690727f633be8c0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258200a4784e70f6d6c05527926f2ca5a0b698429cfaf445889d01e5c4af344c65de50082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820a344a87c16823d5bb509699fa3bc14d4a2532ad4236d11763e5022476020233b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208d7d59e3e5705a5b4c5cf0e3dc50ffb2cf30c7c2ce30fb2e232459bf5c755d0a0082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258201b14f36df5f17fdb543246d2ad019431473e6fdd39be1599b52f68605d0fd0c40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e178ea9c00d9009476b321e6de55aca918c2130c965475471cd45c937f64fff40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258207e0fd50e46b874e9754d520c4097b93ec9af7e152b19b032567ee30bd555f4970082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f53490158948282582099c66e979975ec4ae94487bcfca40372738d0879b79e2d6a9beba41deabff0e50082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0011d28aa1581c0e11a695e4fd6e28dbe13609e59989c2e3fd73b8d17dcd6638ded4dca14b64697363324e465431303001588e82825820fa4ed66b88147b1ad7d8774f67490ff22e3157fc5babbd03c5e947891d5087630082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206a41da9bdb24cac56af60432fe5b9418a8aede9623e2997ce7a469032916899b0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258204dde9003d57183d23ff19cfd4275dc8136e695703c9d74d9015d8d634bc6a02d0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582046f8f9a4a5e39df190d9083bdf6f3e4383cd49fb60ed6ecc546f01eed7b2078c0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c25bb18afe488cff815644a91e283e870c2d12212b484a8b8038a1db237f12d80082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c22795d4c0a4c423451cc9d669640daab6d9b5e0aaafaa54142a343dd66e59330082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901c482825820091c4a9d61ce3b94a4b0e6d1800231250a2b0a7401b55eeff231a6fabf0732c3008258390011d461e926a7b78335b9c4034105a29609651effa4fea1afc1abc55058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0038d318a1581c4c5ac6739376849c917d299a4ef3c74b44cfb1a0ebd4948877058559b8194a45617274684e6f646531014a45617274684e6f646532014a45617274684e6f646533014a45617274684e6f646534014a45617274684e6f646535014a45617274684e6f646536014a45617274684e6f646537014a45617274684e6f646538014a45617274684e6f646539014b45617274684e6f64653130014b45617274684e6f64653131014b45617274684e6f64653132014b45617274684e6f64653133014b45617274684e6f64653134014b45617274684e6f64653135014b45617274684e6f64653136014b45617274684e6f64653137014b45617274684e6f64653138014b45617274684e6f64653139014b45617274684e6f64653230014b45617274684e6f64653231014b45617274684e6f64653232014b45617274684e6f64653233014b45617274684e6f64653234014b45617274684e6f6465323501588d828258201b07f1152e52ce0a9dbb561aa2e2d1750ca3a1a4141150a8bad342947a66a3a60182583900e5b1f1f8be0ce783c4794b6567e21d690bfbc8ea6c297a8603b3ad0858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00155cc0a1581cc2f5ddefa7f7e091f202828bf3692ac5c39833068aacf5cdfebbebdaa1444e46543201")?;
        let network = crate::NetworkIdKind::Testnet;
        let addr = Address::from_bech32("addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2")?;
        let gtxd: crate::TxData = crate::TxData::new(
            None,
            vec![addr.clone()],
            Some(reward_address_from_address(&addr).unwrap()),
            inputs,
            network,
            120000,
        )?;

        let uw = TransWallet::new(&addr, &gtxd.get_inputs());
        wallets.add_wallet(&uw);

        // build tx
        let txb_param: (&StandardTxData, &TransWallets, &Address) =
            (&std_asset_txd, &wallets, &addr);
        let standard_tx_builder = AtSATBuilder::new(txb_param);
        let txbuilder = crate::TxBuilder::new(&gtxd, &vec![]);
        let bld_tx = txbuilder.build(&standard_tx_builder).await.unwrap();
        let tx_org = crate::clib::Transaction::new(
            &bld_tx.get_tx_body_typed(),
            &bld_tx.get_txwitness_typed(),
            Some(bld_tx.get_metadata_typed()),
        );
        // println!(
        //     "\nOriginal CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );

        let tx_restored: crate::Transaction =
            crate::clib::Transaction::from_bytes(hex::decode("84a5008282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e192701018282581d60a3527f67e636f3200fef95378e2ef12e86f1a6366cc87734945d46d2821a00989680a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b19019082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a45239789a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a1467444524153411915e0581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea1457454657374195672581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a190244581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b187a021a0002c71d031a0001dbc8075820061796af79604cd0ad69254034fc59b5c976f0fd56db73514dc51285ad0d8602a0f5a100a200781f48656c6c6f204d7920667269656e64207468697320697320666f7220796f7501782148656c6c6f204d7920667269656e64207468697320697320666f7220796f752032")?)?;
        // println!(
        //     "\nRestored CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );
        assert_eq!(
            hex::encode(tx_org.to_bytes()),
            hex::encode(tx_restored.to_bytes())
        );
        std::fs::remove_file(std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH").unwrap()).unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_metadata_for_stdtx_7transfers() -> Result<(), MurinError> {
        let pp = "{
            \"collateralPercentage\": 150,
            \"costModels\": {
                \"PlutusScriptV1\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10},
                \"PlutusScriptV2\": {\"addInteger-cpu-arguments-intercept\":205665,\"addInteger-cpu-arguments-slope\":812,\"addInteger-memory-arguments-intercept\":1,\"addInteger-memory-arguments-slope\":1,\"appendByteString-cpu-arguments-intercept\":1000,\"appendByteString-cpu-arguments-slope\":571,\"appendByteString-memory-arguments-intercept\":0,\"appendByteString-memory-arguments-slope\":1,\"appendString-cpu-arguments-intercept\":1000,\"appendString-cpu-arguments-slope\":24177,\"appendString-memory-arguments-intercept\":4,\"appendString-memory-arguments-slope\":1,\"bData-cpu-arguments\":1000,\"bData-memory-arguments\":32,\"blake2b_256-cpu-arguments-intercept\":117366,\"blake2b_256-cpu-arguments-slope\":10475,\"blake2b_256-memory-arguments\":4,\"cekApplyCost-exBudgetCPU\":23000,\"cekApplyCost-exBudgetMemory\":100,\"cekBuiltinCost-exBudgetCPU\":23000,\"cekBuiltinCost-exBudgetMemory\":100,\"cekConstCost-exBudgetCPU\":23000,\"cekConstCost-exBudgetMemory\":100,\"cekDelayCost-exBudgetCPU\":23000,\"cekDelayCost-exBudgetMemory\":100,\"cekForceCost-exBudgetCPU\":23000,\"cekForceCost-exBudgetMemory\":100,\"cekLamCost-exBudgetCPU\":23000,\"cekLamCost-exBudgetMemory\":100,\"cekStartupCost-exBudgetCPU\":100,\"cekStartupCost-exBudgetMemory\":100,\"cekVarCost-exBudgetCPU\":23000,\"cekVarCost-exBudgetMemory\":100,\"chooseData-cpu-arguments\":19537,\"chooseData-memory-arguments\":32,\"chooseList-cpu-arguments\":175354,\"chooseList-memory-arguments\":32,\"chooseUnit-cpu-arguments\":46417,\"chooseUnit-memory-arguments\":4,\"consByteString-cpu-arguments-intercept\":221973,\"consByteString-cpu-arguments-slope\":511,\"consByteString-memory-arguments-intercept\":0,\"consByteString-memory-arguments-slope\":1,\"constrData-cpu-arguments\":89141,\"constrData-memory-arguments\":32,\"decodeUtf8-cpu-arguments-intercept\":497525,\"decodeUtf8-cpu-arguments-slope\":14068,\"decodeUtf8-memory-arguments-intercept\":4,\"decodeUtf8-memory-arguments-slope\":2,\"divideInteger-cpu-arguments-constant\":196500,\"divideInteger-cpu-arguments-model-arguments-intercept\":453240,\"divideInteger-cpu-arguments-model-arguments-slope\":220,\"divideInteger-memory-arguments-intercept\":0,\"divideInteger-memory-arguments-minimum\":1,\"divideInteger-memory-arguments-slope\":1,\"encodeUtf8-cpu-arguments-intercept\":1000,\"encodeUtf8-cpu-arguments-slope\":28662,\"encodeUtf8-memory-arguments-intercept\":4,\"encodeUtf8-memory-arguments-slope\":2,\"equalsByteString-cpu-arguments-constant\":245000,\"equalsByteString-cpu-arguments-intercept\":216773,\"equalsByteString-cpu-arguments-slope\":62,\"equalsByteString-memory-arguments\":1,\"equalsData-cpu-arguments-intercept\":1060367,\"equalsData-cpu-arguments-slope\":12586,\"equalsData-memory-arguments\":1,\"equalsInteger-cpu-arguments-intercept\":208512,\"equalsInteger-cpu-arguments-slope\":421,\"equalsInteger-memory-arguments\":1,\"equalsString-cpu-arguments-constant\":187000,\"equalsString-cpu-arguments-intercept\":1000,\"equalsString-cpu-arguments-slope\":52998,\"equalsString-memory-arguments\":1,\"fstPair-cpu-arguments\":80436,\"fstPair-memory-arguments\":32,\"headList-cpu-arguments\":43249,\"headList-memory-arguments\":32,\"iData-cpu-arguments\":1000,\"iData-memory-arguments\":32,\"ifThenElse-cpu-arguments\":80556,\"ifThenElse-memory-arguments\":1,\"indexByteString-cpu-arguments\":57667,\"indexByteString-memory-arguments\":4,\"lengthOfByteString-cpu-arguments\":1000,\"lengthOfByteString-memory-arguments\":10,\"lessThanByteString-cpu-arguments-intercept\":197145,\"lessThanByteString-cpu-arguments-slope\":156,\"lessThanByteString-memory-arguments\":1,\"lessThanEqualsByteString-cpu-arguments-intercept\":197145,\"lessThanEqualsByteString-cpu-arguments-slope\":156,\"lessThanEqualsByteString-memory-arguments\":1,\"lessThanEqualsInteger-cpu-arguments-intercept\":204924,\"lessThanEqualsInteger-cpu-arguments-slope\":473,\"lessThanEqualsInteger-memory-arguments\":1,\"lessThanInteger-cpu-arguments-intercept\":208896,\"lessThanInteger-cpu-arguments-slope\":511,\"lessThanInteger-memory-arguments\":1,\"listData-cpu-arguments\":52467,\"listData-memory-arguments\":32,\"mapData-cpu-arguments\":64832,\"mapData-memory-arguments\":32,\"mkCons-cpu-arguments\":65493,\"mkCons-memory-arguments\":32,\"mkNilData-cpu-arguments\":22558,\"mkNilData-memory-arguments\":32,\"mkNilPairData-cpu-arguments\":16563,\"mkNilPairData-memory-arguments\":32,\"mkPairData-cpu-arguments\":76511,\"mkPairData-memory-arguments\":32,\"modInteger-cpu-arguments-constant\":196500,\"modInteger-cpu-arguments-model-arguments-intercept\":453240,\"modInteger-cpu-arguments-model-arguments-slope\":220,\"modInteger-memory-arguments-intercept\":0,\"modInteger-memory-arguments-minimum\":1,\"modInteger-memory-arguments-slope\":1,\"multiplyInteger-cpu-arguments-intercept\":69522,\"multiplyInteger-cpu-arguments-slope\":11687,\"multiplyInteger-memory-arguments-intercept\":0,\"multiplyInteger-memory-arguments-slope\":1,\"nullList-cpu-arguments\":60091,\"nullList-memory-arguments\":32,\"quotientInteger-cpu-arguments-constant\":196500,\"quotientInteger-cpu-arguments-model-arguments-intercept\":453240,\"quotientInteger-cpu-arguments-model-arguments-slope\":220,\"quotientInteger-memory-arguments-intercept\":0,\"quotientInteger-memory-arguments-minimum\":1,\"quotientInteger-memory-arguments-slope\":1,\"remainderInteger-cpu-arguments-constant\":196500,\"remainderInteger-cpu-arguments-model-arguments-intercept\":453240,\"remainderInteger-cpu-arguments-model-arguments-slope\":220,\"remainderInteger-memory-arguments-intercept\":0,\"remainderInteger-memory-arguments-minimum\":1,\"remainderInteger-memory-arguments-slope\":1,\"serialiseData-cpu-arguments-intercept\":1159724,\"serialiseData-cpu-arguments-slope\":392670,\"serialiseData-memory-arguments-intercept\":0,\"serialiseData-memory-arguments-slope\":2,\"sha2_256-cpu-arguments-intercept\":806990,\"sha2_256-cpu-arguments-slope\":30482,\"sha2_256-memory-arguments\":4,\"sha3_256-cpu-arguments-intercept\":1927926,\"sha3_256-cpu-arguments-slope\":82523,\"sha3_256-memory-arguments\":4,\"sliceByteString-cpu-arguments-intercept\":265318,\"sliceByteString-cpu-arguments-slope\":0,\"sliceByteString-memory-arguments-intercept\":4,\"sliceByteString-memory-arguments-slope\":0,\"sndPair-cpu-arguments\":85931,\"sndPair-memory-arguments\":32,\"subtractInteger-cpu-arguments-intercept\":205665,\"subtractInteger-cpu-arguments-slope\":812,\"subtractInteger-memory-arguments-intercept\":1,\"subtractInteger-memory-arguments-slope\":1,\"tailList-cpu-arguments\":41182,\"tailList-memory-arguments\":32,\"trace-cpu-arguments\":212342,\"trace-memory-arguments\":32,\"unBData-cpu-arguments\":31220,\"unBData-memory-arguments\":32,\"unConstrData-cpu-arguments\":32696,\"unConstrData-memory-arguments\":32,\"unIData-cpu-arguments\":43357,\"unIData-memory-arguments\":32,\"unListData-cpu-arguments\":32247,\"unListData-memory-arguments\":32,\"unMapData-cpu-arguments\":38314,\"unMapData-memory-arguments\":32,\"verifyEcdsaSecp256k1Signature-cpu-arguments\":20000000000,\"verifyEcdsaSecp256k1Signature-memory-arguments\":20000000000,\"verifyEd25519Signature-cpu-arguments-intercept\":9462713,\"verifyEd25519Signature-cpu-arguments-slope\":1021,\"verifyEd25519Signature-memory-arguments\":10,\"verifySchnorrSecp256k1Signature-cpu-arguments-intercept\":20000000000,\"verifySchnorrSecp256k1Signature-cpu-arguments-slope\":0,\"verifySchnorrSecp256k1Signature-memory-arguments\":20000000000}
            },
            \"decentralization\": null,
            \"executionUnitPrices\": {
                \"priceMemory\": 5.77e-2,
                \"priceSteps\": 7.21e-5
            },
            \"extraPraosEntropy\": null,
            \"maxBlockBodySize\": 90112,
            \"maxBlockExecutionUnits\": {
                \"memory\": 62000000,
                \"steps\": 40000000000
            },
            \"maxBlockHeaderSize\": 1100,
            \"maxCollateralInputs\": 3,
            \"maxTxExecutionUnits\": {
                \"memory\": 14000000,
                \"steps\": 10000000000
            },
            \"maxTxSize\": 16384,
            \"maxValueSize\": 5000,
            \"minPoolCost\": 340000000,
            \"minUTxOValue\": null,
            \"monetaryExpansion\": 3.0e-3,
            \"poolPledgeInfluence\": 0.3,
            \"poolRetireMaxEpoch\": 18,
            \"protocolVersion\": {
                \"major\": 7,
                \"minor\": 0
            },
            \"stakeAddressDeposit\": 2000000,
            \"stakePoolDeposit\": 500000000,
            \"stakePoolTargetNum\": 500,
            \"treasuryCut\": 0.2,
            \"txFeeFixed\": 155381,
            \"txFeePerByte\": 44,
            \"utxoCostPerByte\": 4310,
            \"utxoCostPerWord\": null
        }";
        std::fs::write("protocol_parameters_babbage_test.json", pp).unwrap();
        set_var(
            "CARDANO_PROTOCOL_PARAMETER_PATH",
            "protocol_parameters_babbage_test.json",
        );
        let std_asset_txd = serde_json::from_str::<StandardTxData>("{
            \"wallet_addresses\": [
                \"addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2\",
                \"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"addr_test1qqdp3cry5vc2gfjljctdu638tvkcqfx40fjunht9hrmru5zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qnaxxgs\",
                \"addr_test1qr2mw080ujz0unmpn9lx5ftfuewc6htyr6v3a0svul2zgezcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qgryf7t\",
                \"addr_test1qr7tqh7tsg4lut3jv6tsfwlv464m6knjjw90ugyz8uzgr6zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qt0jxzj\",
                \"addr_test1qrscurjp292sxv24sepj7ghq4ydkkekzaz53zwfswcna6ljcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6q8pu3l5\",
                \"addr_test1qqssrphse6qmp9h0ksu5vfmsx99tfl2lc6rhvy2spd5wr86cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw59j4j\",
                \"addr_test1qqgagc0fy6nm0qe4h8zqxsg952tqjeg7l7j0agd0cx4u25zcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qxvept2\",
                \"addr_test1qrjmru0chcxw0q7y099k2elzr45sh77gafkzj75xqwe66zzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qthp2py\",
                \"addr_test1qq78tygxpu7a53rz7m6jnrtf5s8sc6dvg63jz80uyqrfswzcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qhe9ekw\",
                \"addr_test1qpvntdhn6s9d09z72f75atv8ha8qax46a5tfpcf7cp2jwm6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6ql87t33\",
                \"addr_test1qqrja5l2hdl5gdyz7xvm948jg7vc9ed0uzp28yqgveaxww6cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qkw5xzz\",
                \"addr_test1qpzmsl9qfyzlh94049ya2ffjy8akvhmrhc6azdccmdyn2j2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qzudgs9\"
            ],
            \"transfers\":[
            {
                \"receiver\":\"addr_test1vz34ylm8ucm0xgq0a72n0r3w7yhgdudxxekvsae5j3w5d5sje670h\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you\"
            },
            {
                \"receiver\":\"addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 2\"
            },
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 3\"
            },
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 4\"
            },
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 5\"
            },
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 6\"
            },
            {
                \"receiver\":\"addr_test1qpg8ehvgj9zxrx59et72yjn2p02xwsm3l89jwj8ujcj63ujcer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qw23emu\",
                \"assets\" : [
                    {
                        \"amount\": \"5000000\"
                    },
                    {
                        \"fingerprint\":\"asset10q8zsrnx5plw0k2l2e8slcjf4htuvu42jxrgl8\",
                        \"policy\":\"dfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9\",
                        \"tokenname\":\"7441726b\",
                        \"amount\": \"200\"
                    }
                ],
                \"metadata\" : \"Hello My friend this is for you 7\"
            }
        ]
        }")?;
        let mut wallets = TransWallets::new();

        // Input is from DBsync so we have live data, better would be a fixed data set
        let inputs = TransactionUnspentOutputs::from_hex("9828588e828258203182c2a0a4d98cf4fe8e491cbf9068e43b100842eec7cbe3319b9f4b16e8fa820082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582044b6f488071f1e105f709a2f0df98b465331b73e8374be56f33af703589ccc0f0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588f8282582045c6b0ddcf6b58d91ae99671fc8efca89abbb60826871db3eb33c97f9684bff80082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004949b3a1581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a19096d588e8282582033ba220a75ca5734bdc041f0f6005f69db0cce225efa98af1cd74f747873a69b0082583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901078282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a22df7a93a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a146744452415341190af0581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea1457454657374192b39581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a190122581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b190105588e82825820f0de9976070444386a4070434cc18f21c6dd0cd3c53b49050c3dcec84d6535570082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258209907fcf5847e61f3724aa2c3165b42eb37912620bd6e3da6f119fae4f3418e710082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820aeaab7142e18485b35b473122499913eafa3613b353cdb5a013f1875ce3884d50082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c29470125cc1663361a1867494a1c9f39a4ae267b1aa55f7ff87ef0468aeee4c0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208ae4fa04190022b870725f075eadd01c55ffa3a4d67f21356ae0310ded03f81f0082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270082583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a004c4b40a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b18c8588e82825820f67ac37cf0a91615fc2d8415835921650a9217c36df62e1321d1271519d2194500825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ea6ebb93e208716cb3cb0cac47066f2a47a4e472274483fdc18901228596754900825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e037166b40a9fbcf51003ddee74c7f71d25f0e30e55992a12bc792692f2585e100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582005e27d72124d535a80439710c77cb55cf406962b0a92c6c928f9f635b0fa46d700825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015899828258200ca67284834d0714809151fc8b8e8c7747e119786e5a734949f46391b024122400825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a001226b8a1581c868f75968b696aa86be159ea63f31221df4cd4b7a48159fb632968b3a24774656e4e465433014774656e4e46543501588e8282582075710b4252f599a97f9d05e4baafbd2d78fffc79be3879c94ef77ba8626eba1100825839001a18e064a330a4265f9616de6a275b2d8024d57a65c9dd65b8f63e5058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820b9c3ac99ffcb6eaa806609bbb9350d7369be837f1a58972ebec6225f935d5b2e0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e8d6059bd8627cd69f31445bc03d0c5221d9130fed7e47cea674e61c29302c8a0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820f0d8fe5ab94aacd7901cdbca325b5003d4767bef5093441abbcdfe3e1f345c1b0082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206be7d565e9844c813f71e76fa0daa99be67d5d558eeb7d6cb5483f7cb843b1310082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820ff79939f8235e2b176b5673fb0ecbdbf3299870f80ac075248ceff8bdeeabc040082583900d5b73cefe484fe4f61997e6a2569e65d8d5d641e991ebe0ce7d4246458c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206ef58e3560d3e490eef8da9e32bc0e2ae6cf7f8f213e3ae3d97ebe795be4989d0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820940e7bdb43b3e2242918e79afe478ba7d45a6467cf8388d90de36b17f8834a2b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582054f16809c7c94af95af7550c5d720d0342474b2da1a73a753f690727f633be8c0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258200a4784e70f6d6c05527926f2ca5a0b698429cfaf445889d01e5c4af344c65de50082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820a344a87c16823d5bb509699fa3bc14d4a2532ad4236d11763e5022476020233b0082583900fcb05fcb822bfe2e32669704bbecaeabbd5a72938afe20823f0481e858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258208d7d59e3e5705a5b4c5cf0e3dc50ffb2cf30c7c2ce30fb2e232459bf5c755d0a0082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258201b14f36df5f17fdb543246d2ad019431473e6fdd39be1599b52f68605d0fd0c40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820e178ea9c00d9009476b321e6de55aca918c2130c965475471cd45c937f64fff40082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258207e0fd50e46b874e9754d520c4097b93ec9af7e152b19b032567ee30bd555f4970082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f53490158948282582099c66e979975ec4ae94487bcfca40372738d0879b79e2d6a9beba41deabff0e50082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0011d28aa1581c0e11a695e4fd6e28dbe13609e59989c2e3fd73b8d17dcd6638ded4dca14b64697363324e465431303001588e82825820fa4ed66b88147b1ad7d8774f67490ff22e3157fc5babbd03c5e947891d5087630082583900e18e0e41515503315586432f22e0a91b6b66c2e8a91139307627dd7e58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258206a41da9bdb24cac56af60432fe5b9418a8aede9623e2997ce7a469032916899b0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e828258204dde9003d57183d23ff19cfd4275dc8136e695703c9d74d9015d8d634bc6a02d0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e8282582046f8f9a4a5e39df190d9083bdf6f3e4383cd49fb60ed6ecc546f01eed7b2078c0082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c25bb18afe488cff815644a91e283e870c2d12212b484a8b8038a1db237f12d80082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f534901588e82825820c22795d4c0a4c423451cc9d669640daab6d9b5e0aaafaa54142a343dd66e59330082583900210186f0ce81b096efb439462770314ab4fd5fc6877611500b68e19f58c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00118f32a1581cd35c752af635d9e9cb79aea44537a57a5ecd91e23133cd7f210f0070a1456d544f5349015901c482825820091c4a9d61ce3b94a4b0e6d1800231250a2b0a7401b55eeff231a6fabf0732c3008258390011d461e926a7b78335b9c4034105a29609651effa4fea1afc1abc55058c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a0038d318a1581c4c5ac6739376849c917d299a4ef3c74b44cfb1a0ebd4948877058559b8194a45617274684e6f646531014a45617274684e6f646532014a45617274684e6f646533014a45617274684e6f646534014a45617274684e6f646535014a45617274684e6f646536014a45617274684e6f646537014a45617274684e6f646538014a45617274684e6f646539014b45617274684e6f64653130014b45617274684e6f64653131014b45617274684e6f64653132014b45617274684e6f64653133014b45617274684e6f64653134014b45617274684e6f64653135014b45617274684e6f64653136014b45617274684e6f64653137014b45617274684e6f64653138014b45617274684e6f64653139014b45617274684e6f64653230014b45617274684e6f64653231014b45617274684e6f64653232014b45617274684e6f64653233014b45617274684e6f64653234014b45617274684e6f6465323501588d828258201b07f1152e52ce0a9dbb561aa2e2d1750ca3a1a4141150a8bad342947a66a3a60182583900e5b1f1f8be0ce783c4794b6567e21d690bfbc8ea6c297a8603b3ad0858c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a00155cc0a1581cc2f5ddefa7f7e091f202828bf3692ac5c39833068aacf5cdfebbebdaa1444e46543201")?;
        let network = crate::NetworkIdKind::Testnet;
        let addr = Address::from_bech32("addr_test1qqt86eq9972q3qttj6ztje97llasktzfzvhmdccqjlqjaq2cer3t74yn0dm8xqnr7rtwhkqcrpsmphwcf0mlmn39ry6qy6q5t2")?;
        let gtxd: crate::TxData = crate::TxData::new(
            None,
            vec![addr.clone()],
            Some(reward_address_from_address(&addr).unwrap()),
            inputs,
            network,
            120000,
        )?;

        let uw = TransWallet::new(&addr, &gtxd.get_inputs());
        wallets.add_wallet(&uw);

        // build tx
        let txb_param: (&StandardTxData, &TransWallets, &Address) =
            (&std_asset_txd, &wallets, &addr);
        let standard_tx_builder = AtSATBuilder::new(txb_param);
        let txbuilder = crate::TxBuilder::new(&gtxd, &vec![]);
        let bld_tx = txbuilder.build(&standard_tx_builder).await.unwrap();
        let tx_org = crate::clib::Transaction::new(
            &bld_tx.get_tx_body_typed(),
            &bld_tx.get_txwitness_typed(),
            Some(bld_tx.get_metadata_typed()),
        );
        // println!(
        //     "\nOriginal CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );
        let tx_restored: crate::Transaction =
            crate::clib::Transaction::from_bytes(hex::decode("84a5008782582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e19270182582061be7aa47507fc4d28472b80e7b40560e18a7d12dce1fe9a3d1ebb12e72e192701018382581d60a3527f67e636f3200fef95378e2ef12e86f1a6366cc87734945d46d2821a004c4b40a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b18c882583900167d64052f9408816b9684b964befffb0b2c49132fb6e30097c12e8158c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821af24f7ef8a4581c3f1d9bd2f8c3d7d8144b789433261370eaf25cdc83fe8a745ef880c1a146744452415341194c90581cc693a41d2b4f241c992b88c7238131d92202206ffc92f5eae090d0eea14574546573741a00012e8f581cd25b21f3694dfe8b614bad38b149281735a59a04c76353bc72fbbfbfa14474464c5a1907ee581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b19027382583900507cdd889144619a85cafca24a6a0bd4674371f9cb2748fc9625a8f258c8e2bf54937b76730263f0d6ebd8181861b0ddd84bf7fdce251934821a017d7840a1581cdfd18a815a25339777dcc80bce9c438ad632272d95f334a111711ac9a1447441726b1903e8021a0003178d031a0001dbc80758202d673ccb70a500f4410734086ff191c2919616506bd6b49b4b834b5720d5c505a0f5a100a700781f48656c6c6f204d7920667269656e64207468697320697320666f7220796f7501782148656c6c6f204d7920667269656e64207468697320697320666f7220796f75203202782148656c6c6f204d7920667269656e64207468697320697320666f7220796f75203303782148656c6c6f204d7920667269656e64207468697320697320666f7220796f75203404782148656c6c6f204d7920667269656e64207468697320697320666f7220796f75203505782148656c6c6f204d7920667269656e64207468697320697320666f7220796f75203606782148656c6c6f204d7920667269656e64207468697320697320666f7220796f752037")?)?;
        // println!(
        //     "\nRestored CBOR transaction:\n{:?}",
        //     hex::encode(tx_org.to_bytes())
        // );
        assert_eq!(
            hex::encode(tx_org.to_bytes()),
            hex::encode(tx_restored.to_bytes())
        );
        std::fs::remove_file(std::env::var("CARDANO_PROTOCOL_PARAMETER_PATH").unwrap()).unwrap();
        Ok(())
    }
}
