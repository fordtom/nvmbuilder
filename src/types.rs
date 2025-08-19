use crate::error::*;
use crate::variants::DataSheet;
use indexmap::IndexMap;
use serde::Deserialize;

/// Top level struct that contains the settings and the block.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(flatten)]
    pub blocks: IndexMap<String, Block>,
}

/// Function to provide a default padding value.
fn default_padding() -> u8 {
    0xFF
}

/// Endianness enum.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Endianness {
    Little,
    Big,
}

/// CRC settings.
#[derive(Debug, Deserialize)]
pub struct CrcData {
    pub polynomial: u32,
    pub start: u32,
    pub xor_out: u32,
    pub reverse: bool,
}

/// High-level settings.
#[derive(Debug, Deserialize)]
pub struct Settings {
    pub endianness: Endianness,
    #[serde(default = "default_padding")]
    pub padding: u8,
    pub crc: CrcData,
}

/// Flash block.
#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: Header,
    pub data: Entry,
}

/// Flash block header.
#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    pub crc_location: u32,
    pub padding: Option<u8>,
}

/// Scalar type enum derived from 'type' string in leaf entries.
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
    /// Returns the size of the scalar type in bytes.
    pub fn size_bytes(&self) -> usize {
        match self {
            ScalarType::U8 | ScalarType::I8 => 1,
            ScalarType::U16 | ScalarType::I16 => 2,
            ScalarType::U32 | ScalarType::I32 | ScalarType::F32 => 4,
            ScalarType::U64 | ScalarType::I64 | ScalarType::F64 => 8,
        }
    }
}

/// Value representation enum.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DataValue {
    U64(u64),
    I64(i64),
    F64(f64),
    Str(String),
}

impl DataValue {
    pub fn to_bytes(
        &self,
        scalar_type: ScalarType,
        endianness: &Endianness,
    ) -> Result<Vec<u8>, NvmError> {
        match scalar_type {
            ScalarType::U8 => Ok(u8::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::I8 => Ok(i8::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::U16 => Ok(u16::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::I16 => Ok(i16::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::U32 => Ok(u32::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::I32 => Ok(i32::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::U64 => Ok(u64::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::I64 => Ok(i64::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::F32 => Ok(f32::try_from(self)?.to_endian_bytes(endianness)),
            ScalarType::F64 => Ok(f64::try_from(self)?.to_endian_bytes(endianness)),
        }
    }
}

/// Value source struct - necessary for making name/value mutually exclusive.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueSource {
    Single(DataValue),
    Array(Vec<DataValue>),
}

/// Mutually exclusive source enum.
#[derive(Debug, Deserialize)]
// #[serde(untagged)]
pub enum EntrySource {
    #[serde(rename = "name")]
    Name(String),
    #[serde(rename = "value")]
    Value(ValueSource),
}

/// Size source enum.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SizeSource {
    OneD(usize),
    TwoD([usize; 2]),
}

/// Leaf entry representing an item to add to the flash block.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeafEntry {
    #[serde(rename = "type")]
    pub scalar_type: ScalarType,
    #[serde(default)]
    pub size: Option<SizeSource>,
    #[serde(flatten)]
    pub source: EntrySource,
}

/// Any entry - should always be either a leaf or a branch (more entries).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Leaf(LeafEntry),
    Branch(IndexMap<String, Entry>),
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

    pub fn emit_bytes(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
    ) -> Result<Vec<u8>, NvmError> {
        match self.size {
            None => self.emit_bytes_single(data_sheet, endianness),
            // Some(SizeSource::OneD(size)) => {
            //     let value = match self.source {
            //         EntrySource::Value(value) => value.value,
            //         EntrySource::Name(name) => data_sheet.retrieve_1d_array_or_string(&name)?,
            //     };
            // }
            // Some(SizeSource::TwoD(size)) => {}
            _ => {
                return Err(NvmError::DataValueExportFailed(
                    "2D arrays are not supported yet.".to_string(),
                ));
            }
        }
    }

    fn emit_bytes_single(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
    ) -> Result<Vec<u8>, NvmError> {
        match &self.source {
            EntrySource::Name(name) => {
                let value = data_sheet.retrieve_single_value(name)?;
                value.to_bytes(self.scalar_type, endianness)
            }
            EntrySource::Value(value) => match value {
                ValueSource::Single(value) => value.to_bytes(self.scalar_type, endianness),
                _ => {
                    return Err(NvmError::DataValueExportFailed(
                        "Single value expected for scalar type.".to_string(),
                    ))
                }
            },
        }
    }

    /// Returns the alignment of the leaf entry.
    pub fn get_alignment(&self) -> usize {
        self.scalar_type.size_bytes()
    }
}

trait EndianBytes {
    fn to_endian_bytes(self, endianness: &Endianness) -> Vec<u8>;
}

macro_rules! impl_endian_bytes {
    ($($t:ty),* $(,)?) => {$(
        impl EndianBytes for $t {
            fn to_endian_bytes(self, e: &Endianness) -> Vec<u8> {
                match e {
                    Endianness::Little => self.to_le_bytes().to_vec(),
                    Endianness::Big => self.to_be_bytes().to_vec(),
                }
            }
        }
    )*};
}
impl_endian_bytes!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

macro_rules! impl_try_from_data_value {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<&DataValue> for $t {
            type Error = NvmError;
            fn try_from(value: &DataValue) -> Result<Self, NvmError> {
                match value {
                    DataValue::U64(val) => Ok(*val as $t),
                    DataValue::I64(val) => Ok(*val as $t),
                    DataValue::F64(val) => Ok(*val as $t),
                    DataValue::Str(val) => {
                        return Err(NvmError::DataValueExportFailed(
                            "Cannot convert string to scalar type.".to_string(),
                        ));
                    }
                }
            }
        }
    )* }; }

impl_try_from_data_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
