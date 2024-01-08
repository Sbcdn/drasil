use cardano_serialization_lib::error::DeserializeError;
use cardano_serialization_lib::error::DeserializeFailure;
use cardano_serialization_lib::error::JsError;
use cardano_serialization_lib::error::Key;
use cardano_serialization_lib::utils;
use cardano_serialization_lib::utils::Int;
use cbor_event::Serialize;
use cbor_event::Special as CBORSpecial;
use cbor_event::Type as CBORType;
use cbor_event::{de::Deserializer, se::Serializer};
use std::io::{BufRead, Seek, Write};

use super::binary;
use super::binary::*;

const PLUTUS_V1_COST_MODEL_OP_COUNT: usize = 166;
const PLUTUS_V2_COST_MODEL_OP_COUNT: usize = 175;

fn cost_model_op_count(lang: LanguageKind) -> usize {
    match lang {
        LanguageKind::PlutusV1 => PLUTUS_V1_COST_MODEL_OP_COUNT,
        LanguageKind::PlutusV2 => PLUTUS_V2_COST_MODEL_OP_COUNT,
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CostModel {
    language: Language,
    pub op_costs: Vec<Int>,
}

binary::to_from_bytes!(CostModel);

impl CostModel {
    pub fn empty_model(language: &Language) -> Self {
        let op_count = cost_model_op_count(language.0);
        let mut op_costs = Vec::with_capacity(op_count);
        for _ in 0..op_count {
            op_costs.push(Int::new_i32(0));
        }
        Self {
            language: language.clone(),
            op_costs,
        }
    }

    pub fn set(&mut self, operation: usize, cost: &Int) -> Result<Int, JsError> {
        if operation >= self.op_costs.len() {
            return Err(JsError::from_str(&format!(
                "CostModel operation {} out of bounds. Max is {}",
                operation,
                self.op_costs.len()
            )));
        }
        let old = self.op_costs[operation].clone();
        self.op_costs[operation] = cost.clone();
        Ok(old)
    }

    pub fn get(&self, operation: usize) -> Result<Int, JsError> {
        if operation >= self.op_costs.len() {
            return Err(JsError::from_str(&format!(
                "CostModel operation {} out of bounds. Max is {}",
                operation,
                self.op_costs.len()
            )));
        }
        Ok(self.op_costs[operation].clone())
    }

    pub fn language(&self) -> Language {
        self.language.clone()
    }
}
impl CostModel {
    pub fn new(language: &Language, op_costs: &Vec<Int>) -> Self {
        Self {
            language: language.clone(),
            op_costs: op_costs.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Costmdls(std::collections::BTreeMap<Language, CostModel>);

binary::to_from_bytes!(Costmdls);

impl Costmdls {
    pub fn new() -> Self {
        Self(std::collections::BTreeMap::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, value: &CostModel) -> Option<CostModel> {
        self.0.insert(value.language.clone(), value.clone())
    }

    pub fn get(&self, key: &Language) -> Option<CostModel> {
        self.0.get(key).map(|v| v.clone())
    }

    pub fn keys(&self) -> Languages {
        Languages(self.0.iter().map(|(k, _v)| k.clone()).collect::<Vec<_>>())
    }

    pub(crate) fn language_views_encoding(&self) -> Vec<u8> {
        let mut serializer = Serializer::new_vec();
        let mut keys_bytes: Vec<(Language, Vec<u8>)> = self
            .0
            .iter()
            .map(|(k, _v)| (k.clone(), k.to_bytes()))
            .collect();
        // keys must be in canonical ordering first
        keys_bytes.sort_by(|lhs, rhs| match lhs.1.len().cmp(&rhs.1.len()) {
            std::cmp::Ordering::Equal => lhs.1.cmp(&rhs.1),
            len_order => len_order,
        });
        serializer
            .write_map(cbor_event::Len::Len(self.0.len() as u64))
            .unwrap();
        for (key, key_bytes) in keys_bytes.iter() {
            match key.0 {
                LanguageKind::PlutusV1 => {
                    // For PlutusV1 (language id 0), the language view is the following:
                    //   * the value of costmdls map at key 0 is encoded as an indefinite length
                    //     list and the result is encoded as a bytestring. (our apologies)
                    //   * the language ID tag is also encoded twice. first as a uint then as
                    //     a bytestring. (our apologies)
                    serializer.write_bytes(key_bytes).unwrap();
                    let cost_model = self.0.get(&key).unwrap();
                    // Due to a bug in the cardano-node input-output-hk/cardano-ledger-specs/issues/2512
                    // we must use indefinite length serialization in this inner bytestring to match it
                    let mut cost_model_serializer = Serializer::new_vec();
                    cost_model_serializer
                        .write_array(cbor_event::Len::Indefinite)
                        .unwrap();
                    for cost in &cost_model.op_costs {
                        cost.serialize(&mut cost_model_serializer).unwrap();
                    }
                    cost_model_serializer
                        .write_special(cbor_event::Special::Break)
                        .unwrap();
                    serializer
                        .write_bytes(cost_model_serializer.finalize())
                        .unwrap();
                }
                LanguageKind::PlutusV2 => {
                    // For PlutusV2 (language id 1), the language view is the following:
                    //    * the value of costmdls map at key 1 is encoded as an definite length list.
                    key.serialize(&mut serializer).unwrap();
                    let cost_model = self.0.get(&key).unwrap();
                    serializer
                        .write_array(cbor_event::Len::Len(cost_model.op_costs.len() as u64))
                        .unwrap();
                    for cost in &cost_model.op_costs {
                        cost.serialize(&mut serializer).unwrap();
                    }
                }
            }
        }
        let out = serializer.finalize();
        out
    }
}

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub enum LanguageKind {
    PlutusV1,
    PlutusV2,
}

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub struct Language(LanguageKind);

binary::to_from_bytes!(Language);

impl Language {
    pub fn new_plutus_v1() -> Self {
        Self(LanguageKind::PlutusV1)
    }

    pub fn new_plutus_v2() -> Self {
        Self(LanguageKind::PlutusV2)
    }

    pub fn kind(&self) -> LanguageKind {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Languages(pub(crate) Vec<Language>);

impl Languages {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, index: usize) -> Language {
        self.0[index]
    }

    pub fn add(&mut self, elem: Language) {
        self.0.push(elem);
    }
}

// Serialization
impl cbor_event::se::Serialize for CostModel {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(self.op_costs.len() as u64))?;
        for cost in &self.op_costs {
            cost.serialize(serializer)?;
        }
        Ok(serializer)
    }
}

impl Deserialize for CostModel {
    fn deserialize<R: BufRead + Seek>(raw: &mut Deserializer<R>) -> Result<Self, DeserializeError> {
        (|| -> Result<_, DeserializeError> {
            let mut op_costs = Vec::new();
            let len = raw.array()?;
            while match len {
                cbor_event::Len::Len(n) => op_costs.len() < n as usize,
                cbor_event::Len::Indefinite => true,
            } {
                if raw.cbor_type()? == CBORType::Special {
                    assert_eq!(raw.special()?, CBORSpecial::Break);
                    break;
                }
                op_costs.push(<Int as utils::Deserialize>::deserialize(raw).unwrap());
            }
            let language = match op_costs.len() {
                PLUTUS_V1_COST_MODEL_OP_COUNT => Ok(Language::new_plutus_v1()),
                PLUTUS_V2_COST_MODEL_OP_COUNT => Ok(Language::new_plutus_v2()),
                _ => Err(DeserializeFailure::NoVariantMatched),
            }?;
            Ok(CostModel { language, op_costs })
        })()
        .map_err(|e| e.annotate("CostModel"))
    }
}

impl cbor_event::se::Serialize for Costmdls {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_map(cbor_event::Len::Len(self.0.len() as u64))?;
        for (key, value) in &self.0 {
            key.serialize(serializer)?;
            value.serialize(serializer)?;
        }
        Ok(serializer)
    }
}

impl Deserialize for Costmdls {
    fn deserialize<R: BufRead + Seek>(raw: &mut Deserializer<R>) -> Result<Self, DeserializeError> {
        let mut table = std::collections::BTreeMap::new();
        (|| -> Result<_, DeserializeError> {
            let len = raw.map()?;
            while match len {
                cbor_event::Len::Len(n) => table.len() < n as usize,
                cbor_event::Len::Indefinite => true,
            } {
                if raw.cbor_type()? == CBORType::Special {
                    assert_eq!(raw.special()?, CBORSpecial::Break);
                    break;
                }
                let key = Language::deserialize(raw)?;
                let value = CostModel::deserialize(raw)?;
                if table.insert(key.clone(), value).is_some() {
                    return Err(DeserializeFailure::DuplicateKey(Key::Str(String::from(
                        "some complicated/unsupported type",
                    )))
                    .into());
                }
            }
            Ok(())
        })()
        .map_err(|e| e.annotate("Costmdls"))?;
        Ok(Self(table))
    }
}

impl cbor_event::se::Serialize for Language {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        match self.0 {
            LanguageKind::PlutusV1 => serializer.write_unsigned_integer(0u64),
            LanguageKind::PlutusV2 => serializer.write_unsigned_integer(1u64),
        }
    }
}

impl Deserialize for Language {
    fn deserialize<R: BufRead + Seek>(raw: &mut Deserializer<R>) -> Result<Self, DeserializeError> {
        (|| -> Result<_, DeserializeError> {
            match raw.unsigned_integer()? {
                0 => Ok(Language::new_plutus_v1()),
                _ => Err(DeserializeError::new(
                    "Language",
                    DeserializeFailure::NoVariantMatched.into(),
                )),
            }
        })()
        .map_err(|e| e.annotate("Language"))
    }
}

impl cbor_event::se::Serialize for Languages {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(self.0.len() as u64))?;
        for element in &self.0 {
            element.serialize(serializer)?;
        }
        Ok(serializer)
    }
}

impl Deserialize for Languages {
    fn deserialize<R: BufRead + Seek>(raw: &mut Deserializer<R>) -> Result<Self, DeserializeError> {
        let mut arr = Vec::new();
        (|| -> Result<_, DeserializeError> {
            let len = raw.array()?;
            while match len {
                cbor_event::Len::Len(n) => arr.len() < n as usize,
                cbor_event::Len::Indefinite => true,
            } {
                if raw.cbor_type()? == CBORType::Special {
                    assert_eq!(raw.special()?, CBORSpecial::Break);
                    break;
                }
                arr.push(Language::deserialize(raw)?);
            }
            Ok(())
        })()
        .map_err(|e| e.annotate("Languages"))?;
        Ok(Self(arr))
    }
}
