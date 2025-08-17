use crate::error::*;
use crate::variants::DataSheet;
use serde::Deserialize;
use std::collections::BTreeMap;

// this is the top level struct that contains the settings and the block
#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(flatten)]
    pub blocks: BTreeMap<String, Block>,
}

fn default_padding() -> u8 {
    0xFF
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub endianness: Endianness,
    pub crc: CrcData,
    #[serde(default = "default_padding")]
    pub padding: u8,
}

#[derive(Debug, Deserialize)]
pub struct CrcData {
    pub polynomial: u32,
    pub start: u32,
    pub xor_out: u32,
    pub reverse: bool,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: Header,
    pub data: Entry,
}

#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    pub crc_location: u32,
    pub padding: Option<u8>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ScalarType {
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "u16")]
    U16,
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "u64")]
    U64,
    #[serde(rename = "i8")]
    I8,
    #[serde(rename = "i16")]
    I16,
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "f32")]
    F32,
    #[serde(rename = "f64")]
    F64,
}

impl ScalarType {
    pub fn size_bytes(&self) -> usize {
        match self {
            ScalarType::U8 | ScalarType::I8 => 1,
            ScalarType::U16 | ScalarType::I16 => 2,
            ScalarType::U32 | ScalarType::I32 | ScalarType::F32 => 4,
            ScalarType::U64 | ScalarType::I64 | ScalarType::F64 => 8,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DataValueRepr {
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueSource {
    pub value: DataValueRepr,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameSource {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum EntrySource {
    Value(ValueSource),
    Name(NameSource),
}

#[derive(Debug, Deserialize)]
pub struct LeafEntry {
    #[serde(rename = "type")]
    pub scalar_type: ScalarType,
    #[serde(flatten)]
    pub source: EntrySource,
    size: Option<[usize; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Leaf(LeafEntry),
    Branch(BTreeMap<String, Entry>),
}

impl LeafEntry {
    // pass ref to vec to avoid copying
    // pub fn emit_bytes(&self, data_sheet: &DataSheet, endianness: &Endianness) -> Vec<u8> {
    //     let value = self.get_value(data_sheet)?;

    //     // as examples for byte export
    //     // (DataValue::F64(val), Endianness::Little) => val.to_le_bytes().to_vec(),
    //     // (DataValue::F64(val), Endianness::Big) => val.to_be_bytes().to_vec(),
    // }

    // fn get_value(&self, data_sheet: &DataSheet) -> Result<DataValue, NvmError> {
    //     match self.source {
    //         EntrySource::Value(value) => Ok(value.value),
    //         EntrySource::Name(name) => data_sheet.retrieve_cell_data(&name, &self.scalar_type),
    //     }
    // }
}
