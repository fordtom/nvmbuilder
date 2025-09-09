use crate::error::*;
use indexmap::IndexMap;
use serde::Deserialize;

/// Top level struct that contains the settings and the block.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(flatten)]
    pub blocks: IndexMap<String, Block>,
}

/// High-level settings.
#[derive(Debug, Deserialize)]
pub struct Settings {
    pub endianness: Endianness,
    pub crc: CrcData,
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
    pub ref_in: bool,
    pub ref_out: bool,
}

/// Flash block.
#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: Header,
    pub data: Entry,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CrcLocation {
    Keyword(String),
    Address(u32),
}

/// Flash block header.
#[derive(Debug, Deserialize)]
pub struct Header {
    pub start_address: u32,
    pub length: u32,
    pub crc_location: CrcLocation,
    #[serde(default = "default_padding")]
    pub padding: u8,
}

/// Function to provide a default padding value.
fn default_padding() -> u8 {
    0xFF
}

/// Any entry - should always be either a leaf or a branch (more entries).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Leaf(LeafEntry),
    Branch(IndexMap<String, Entry>),
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

/// Size source enum.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SizeSource {
    OneD(usize),
    TwoD([usize; 2]),
}

/// Mutually exclusive source enum.
#[derive(Debug, Deserialize)]
pub enum EntrySource {
    #[serde(rename = "name")]
    Name(String),
    #[serde(rename = "value")]
    Value(ValueSource),
}

/// Value source struct - necessary for making name/value mutually exclusive.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueSource {
    Single(DataValue),
    Array(Vec<DataValue>),
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

impl DataValue {
    pub fn to_bytes(
        &self,
        scalar_type: ScalarType,
        endianness: &Endianness,
        strict: bool,
    ) -> Result<Vec<u8>, NvmError> {
        match scalar_type {
            ScalarType::U8 => convert_value_to_bytes::<u8>(self, endianness, strict),
            ScalarType::I8 => convert_value_to_bytes::<i8>(self, endianness, strict),
            ScalarType::U16 => convert_value_to_bytes::<u16>(self, endianness, strict),
            ScalarType::I16 => convert_value_to_bytes::<i16>(self, endianness, strict),
            ScalarType::U32 => convert_value_to_bytes::<u32>(self, endianness, strict),
            ScalarType::I32 => convert_value_to_bytes::<i32>(self, endianness, strict),
            ScalarType::U64 => convert_value_to_bytes::<u64>(self, endianness, strict),
            ScalarType::I64 => convert_value_to_bytes::<i64>(self, endianness, strict),
            ScalarType::F32 => convert_value_to_bytes::<f32>(self, endianness, strict),
            ScalarType::F64 => convert_value_to_bytes::<f64>(self, endianness, strict),
        }
    }

    pub fn string_to_bytes(&self) -> Result<Vec<u8>, NvmError> {
        match self {
            DataValue::Str(val) => Ok(val.as_bytes().to_vec()),
            _ => Err(NvmError::DataValueExportFailed(
                "String expected for string type.".to_string(),
            )),
        }
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

trait LossyFromDataValue: Sized {
    fn lossy_from(value: &DataValue) -> Result<Self, NvmError>;
}

macro_rules! impl_lossy_from_data_value {
    ($($t:ty),* $(,)?) => {$({
        impl LossyFromDataValue for $t {
            fn lossy_from(value: &DataValue) -> Result<Self, NvmError> {
                match value {
                    DataValue::U64(v) => Ok(*v as $t),
                    DataValue::I64(v) => Ok(*v as $t),
                    DataValue::F64(v) => Ok(*v as $t),
                    DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                        "Cannot convert string to scalar type.".to_string(),
                    )),
                }
            }
        }
    })*};
}

impl_lossy_from_data_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

fn convert_value_to_bytes<T>(value: &DataValue, e: &Endianness, strict: bool) -> Result<Vec<u8>, NvmError>
where
    T: EndianBytes + TryFrom<&DataValue, Error = NvmError> + LossyFromDataValue,
{
    let out: T = if strict { T::try_from(value)? } else { T::lossy_from(value)? };
    Ok(out.to_endian_bytes(e))
}

// Strict TryFrom implementations with bound and finiteness checks
impl TryFrom<&DataValue> for u8 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= u8::MAX as u64)
                .then_some(*v as u8)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for u8".to_string())),
            DataValue::I64(v) => (*v >= 0 && *v <= u8::MAX as i64)
                .then_some(*v as u8)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for u8".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < 0.0 || *f > u8::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for u8".to_string()));
                }
                Ok(*f as u8)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for u16 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= u16::MAX as u64)
                .then_some(*v as u16)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for u16".to_string())),
            DataValue::I64(v) => (*v >= 0 && *v <= u16::MAX as i64)
                .then_some(*v as u16)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for u16".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < 0.0 || *f > u16::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for u16".to_string()));
                }
                Ok(*f as u16)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for u32 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= u32::MAX as u64)
                .then_some(*v as u32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for u32".to_string())),
            DataValue::I64(v) => (*v >= 0 && *v <= u32::MAX as i64)
                .then_some(*v as u32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for u32".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < 0.0 || *f > u32::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for u32".to_string()));
                }
                Ok(*f as u32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for u64 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => Ok(*v),
            DataValue::I64(v) => (*v >= 0)
                .then_some(*v as u64)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for u64".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < 0.0 || *f > u64::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for u64".to_string()));
                }
                Ok(*f as u64)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for i8 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= i8::MAX as u64)
                .then_some(*v as i8)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for i8".to_string())),
            DataValue::I64(v) => (*v >= i8::MIN as i64 && *v <= i8::MAX as i64)
                .then_some(*v as i8)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for i8".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < i8::MIN as f64 || *f > i8::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for i8".to_string()));
                }
                Ok(*f as i8)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for i16 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= i16::MAX as u64)
                .then_some(*v as i16)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for i16".to_string())),
            DataValue::I64(v) => (*v >= i16::MIN as i64 && *v <= i16::MAX as i64)
                .then_some(*v as i16)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for i16".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < i16::MIN as f64 || *f > i16::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for i16".to_string()));
                }
                Ok(*f as i16)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for i32 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= i32::MAX as u64)
                .then_some(*v as i32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for i32".to_string())),
            DataValue::I64(v) => (*v >= i32::MIN as i64 && *v <= i32::MAX as i64)
                .then_some(*v as i32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for i32".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < i32::MIN as f64 || *f > i32::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for i32".to_string()));
                }
                Ok(*f as i32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for i64 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => (*v <= i64::MAX as u64)
                .then_some(*v as i64)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for i64".to_string())),
            DataValue::I64(v) => Ok(*v),
            DataValue::F64(f) => {
                if !f.is_finite() || f.fract() != 0.0 || *f < i64::MIN as f64 || *f > i64::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for i64".to_string()));
                }
                Ok(*f as i64)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for f32 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => ((*v as f64) <= f32::MAX as f64)
                .then_some(*v as f32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value too large for f32".to_string())),
            DataValue::I64(v) => ((*v as f64) >= f32::MIN as f64 && (*v as f64) <= f32::MAX as f64)
                .then_some(*v as f32)
                .ok_or_else(|| NvmError::DataValueExportFailed("value out of range for f32".to_string())),
            DataValue::F64(f) => {
                if !f.is_finite() || *f < f32::MIN as f64 || *f > f32::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed("value out of range for f32".to_string()));
                }
                Ok(*f as f32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}

impl TryFrom<&DataValue> for f64 {
    type Error = NvmError;
    fn try_from(value: &DataValue) -> Result<Self, NvmError> {
        match value {
            DataValue::U64(v) => Ok(*v as f64),
            DataValue::I64(v) => Ok(*v as f64),
            DataValue::F64(f) => {
                if !f.is_finite() {
                    return Err(NvmError::DataValueExportFailed("non-finite f64 not allowed".to_string()));
                }
                Ok(*f)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed("Cannot convert string to scalar type.".to_string())),
        }
    }
}
