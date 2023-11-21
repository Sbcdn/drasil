#![allow(non_snake_case)]
use crate::MurinError;
use cardano_serialization_lib::{
    plutus::{CostModel as ClibCostModel, Costmdls, Language},
    utils,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CostModels {
    #[serde(alias = "PlutusScriptV1")]
    plutus_v1: CostModel,
    #[serde(alias = "PlutusScriptV2")]
    plutus_v2: CostModel,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CostModel(serde_json::Value);

//#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
//pub struct CostModels(HashMap<ScriptType, CostModel>);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExUnitPrice {
    pub priceSteps: f64,
    pub priceMemory: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MaxTxExUnits {
    pub memory: f64,
    pub steps: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MaxBlockExUnits {
    pub memory: f64,
    pub steps: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProtocolVersion {
    major: u8,
    minor: u8,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProtocolParameters {
    #[serde(alias = "utxoCostPerByte")]
    pub utxo_cost_per_byte: u64,
    #[serde(alias = "txFeePerByte")]
    pub tx_fee_per_byte: u64,
    #[serde(alias = "txFeeFixed")]
    pub tx_fee_fixed: u64,
    #[serde(alias = "treasuryCut")]
    pub treasury_cut: f32,
    #[serde(alias = "stakePoolTargetNum")]
    pub stake_pool_target_num: u64,
    #[serde(alias = "stakePoolDeposit")]
    pub stake_pool_deposit: i128,
    #[serde(alias = "stakeAddressDeposit")]
    pub stake_address_deposit: u64,
    #[serde(alias = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    #[serde(alias = "poolRetireMaxEpoch")]
    pub pool_retire_max_epoch: i32,
    #[serde(alias = "poolPledgeInfluence")]
    pub pool_pledge_influence: f32,
    #[serde(alias = "monetaryExpansion")]
    pub monetary_expansion: f32,
    #[serde(alias = "minPoolCost")]
    pub min_pool_cost: i128,
    #[serde(alias = "maxValueSize")]
    pub max_value_size: u64,
    #[serde(alias = "maxTxSize")]
    pub max_tx_size: u64,
    #[serde(alias = "maxTxExecutionUnits")]
    pub max_tx_execution_units: MaxTxExUnits,
    #[serde(alias = "maxBlockHeaderSize")]
    pub max_block_header_size: u64,
    #[serde(alias = "maxBlockExecutionUnits")]
    pub max_block_execution_units: MaxBlockExUnits,
    #[serde(alias = "maxBlockBodySize")]
    pub max_block_body_size: u64,
    #[serde(alias = "executionUnitPrices")]
    pub execution_unit_prices: ExUnitPrice,
    #[serde(alias = "collateralPercentage")]
    pub collateral_percentage: u64,
    #[serde(alias = "maxCollateralInputs")]
    pub max_collateral_inputs: i32,
    #[serde(alias = "costModels")]
    pub cost_models: CostModels,
}

impl ProtocolParameters {
    pub fn read_protocol_parameter(path: &String) -> Result<ProtocolParameters, MurinError> {
        ////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        //
        //Set Protocol Parameter
        //
        ////////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // Read txFeePerByte from Protocol Parameters JSON
        let pp_data = std::fs::read_to_string(std::path::PathBuf::from(path))?;
        let pp: ProtocolParameters = serde_json::from_str(&pp_data).unwrap();

        Ok(pp)
    }

    pub fn get_CostMdls(&self) -> Result<Costmdls, MurinError> {
        let mut cstmdls = Costmdls::new();

        let mut v1 = ClibCostModel::new();
        let mut v2 = ClibCostModel::new();
        if self.cost_models.plutus_v1.0.is_object() {
            for (i, (_key, value)) in self
                .cost_models
                .plutus_v1
                .0
                .as_object()
                .unwrap()
                .iter()
                .enumerate()
            {
                v1.set(
                    i,
                    &utils::Int::from_str(&value.as_u64().unwrap().to_string())?,
                )?;
            }
            cstmdls.insert(&Language::new_plutus_v1(), &v1);
        }

        if self.cost_models.plutus_v2.0.is_object() {
            for (i, (_key, value)) in self
                .cost_models
                .plutus_v2
                .0
                .as_object()
                .unwrap()
                .iter()
                .enumerate()
            {
                v2.set(
                    i,
                    &utils::Int::from_str(&value.as_u64().unwrap().to_string())?,
                )?;
            }
            cstmdls.insert(&Language::new_plutus_v2(), &v2);
        }

        Ok(cstmdls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_import() {
        let pp = ProtocolParameters::read_protocol_parameter(
            &"/home/tp/Documents/protocol_parameters_babbage.json".to_owned(),
        )
        .unwrap();

        let a = pp.get_CostMdls().unwrap();

        println!("{:?}", a.to_hex());
    }
}
