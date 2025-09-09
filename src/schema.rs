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
            ScalarType::U8 => Ok(self.to_u8(strict)?.to_endian_bytes(endianness)),
            ScalarType::I8 => Ok(self.to_i8(strict)?.to_endian_bytes(endianness)),
            ScalarType::U16 => Ok(self.to_u16(strict)?.to_endian_bytes(endianness)),
            ScalarType::I16 => Ok(self.to_i16(strict)?.to_endian_bytes(endianness)),
            ScalarType::U32 => Ok(self.to_u32(strict)?.to_endian_bytes(endianness)),
            ScalarType::I32 => Ok(self.to_i32(strict)?.to_endian_bytes(endianness)),
            ScalarType::U64 => Ok(self.to_u64(strict)?.to_endian_bytes(endianness)),
            ScalarType::I64 => Ok(self.to_i64(strict)?.to_endian_bytes(endianness)),
            ScalarType::F32 => Ok(self.to_f32(strict)?.to_endian_bytes(endianness)),
            ScalarType::F64 => Ok(self.to_f64(strict)?.to_endian_bytes(endianness)),
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

    fn to_u8(&self, strict: bool) -> Result<u8, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > u8::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for u8", v),
                    ));
                }
                Ok(*v as u8)
            }
            DataValue::I64(v) => {
                if strict && (*v < 0 || *v > u8::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for u8", v),
                    ));
                }
                Ok(*v as u8)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, 0.0, u8::MAX as f64, "u8")?;
                }
                Ok(*f as u8)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_u16(&self, strict: bool) -> Result<u16, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > u16::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for u16", v),
                    ));
                }
                Ok(*v as u16)
            }
            DataValue::I64(v) => {
                if strict && (*v < 0 || *v > u16::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for u16", v),
                    ));
                }
                Ok(*v as u16)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, 0.0, u16::MAX as f64, "u16")?;
                }
                Ok(*f as u16)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_u32(&self, strict: bool) -> Result<u32, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > u32::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for u32", v),
                    ));
                }
                Ok(*v as u32)
            }
            DataValue::I64(v) => {
                if strict && (*v < 0 || *v > u32::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for u32", v),
                    ));
                }
                Ok(*v as u32)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, 0.0, u32::MAX as f64, "u32")?;
                }
                Ok(*f as u32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_u64(&self, strict: bool) -> Result<u64, NvmError> {
        match self {
            DataValue::U64(v) => Ok(*v),
            DataValue::I64(v) => {
                if strict && *v < 0 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for u64", v),
                    ));
                }
                Ok(*v as u64)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, 0.0, u64::MAX as f64, "u64")?;
                }
                Ok(*f as u64)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_i8(&self, strict: bool) -> Result<i8, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > i8::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for i8", v),
                    ));
                }
                Ok(*v as i8)
            }
            DataValue::I64(v) => {
                if strict && (*v < i8::MIN as i64 || *v > i8::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for i8", v),
                    ));
                }
                Ok(*v as i8)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, i8::MIN as f64, i8::MAX as f64, "i8")?;
                }
                Ok(*f as i8)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_i16(&self, strict: bool) -> Result<i16, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > i16::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for i16", v),
                    ));
                }
                Ok(*v as i16)
            }
            DataValue::I64(v) => {
                if strict && (*v < i16::MIN as i64 || *v > i16::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for i16", v),
                    ));
                }
                Ok(*v as i16)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, i16::MIN as f64, i16::MAX as f64, "i16")?;
                }
                Ok(*f as i16)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_i32(&self, strict: bool) -> Result<i32, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > i32::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for i32", v),
                    ));
                }
                Ok(*v as i32)
            }
            DataValue::I64(v) => {
                if strict && (*v < i32::MIN as i64 || *v > i32::MAX as i64) {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for i32", v),
                    ));
                }
                Ok(*v as i32)
            }
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, i32::MIN as f64, i32::MAX as f64, "i32")?;
                }
                Ok(*f as i32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_i64(&self, strict: bool) -> Result<i64, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && *v > i64::MAX as u64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for i64", v),
                    ));
                }
                Ok(*v as i64)
            }
            DataValue::I64(v) => Ok(*v),
            DataValue::F64(f) => {
                if strict {
                    Self::ensure_finite_integer_in_range(*f, i64::MIN as f64, i64::MAX as f64, "i64")?;
                }
                Ok(*f as i64)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_f32(&self, strict: bool) -> Result<f32, NvmError> {
        match self {
            DataValue::U64(v) => {
                if strict && (*v as f64) > f32::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} too large for f32", v),
                    ));
                }
                Ok(*v as f32)
            }
            DataValue::I64(v) => {
                if strict && (*v as f64) < f32::MIN as f64 || (*v as f64) > f32::MAX as f64 {
                    return Err(NvmError::DataValueExportFailed(
                        format!("value {} out of range for f32", v),
                    ));
                }
                Ok(*v as f32)
            }
            DataValue::F64(f) => {
                if strict {
                    if !f.is_finite() {
                        return Err(NvmError::DataValueExportFailed(
                            "non-finite f64 cannot convert to f32".to_string(),
                        ));
                    }
                    if *f < f32::MIN as f64 || *f > f32::MAX as f64 {
                        return Err(NvmError::DataValueExportFailed(
                            format!("value {} out of range for f32", f),
                        ));
                    }
                }
                Ok(*f as f32)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn to_f64(&self, strict: bool) -> Result<f64, NvmError> {
        match self {
            DataValue::U64(v) => Ok(*v as f64),
            DataValue::I64(v) => Ok(*v as f64),
            DataValue::F64(f) => {
                if strict && !f.is_finite() {
                    return Err(NvmError::DataValueExportFailed(
                        "non-finite f64 not allowed".to_string(),
                    ));
                }
                Ok(*f)
            }
            DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                "Cannot convert string to scalar type.".to_string(),
            )),
        }
    }

    fn ensure_finite_integer_in_range(value: f64, min: f64, max: f64, target: &str) -> Result<(), NvmError> {
        if !value.is_finite() {
            return Err(NvmError::DataValueExportFailed(
                format!("non-finite value cannot convert to {}", target),
            ));
        }
        if value < min || value > max {
            return Err(NvmError::DataValueExportFailed(
                format!("value {} out of range for {}", value, target),
            ));
        }
        // Ensure no fractional part
        if (value.fract()).abs() > 0.0 {
            return Err(NvmError::DataValueExportFailed(
                format!("value {} is not an integer for {}", value, target),
            ));
        }
        Ok(())
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

// Non-strict fallback conversions still use TryFrom<&DataValue> where we allow casting.
// We keep these for the non-strict path in to_bytes.
macro_rules! impl_try_from_data_value {
    ($($t:ty),* $(,)?) => {$({
        impl TryFrom<&DataValue> for $t {
            type Error = NvmError;
            fn try_from(value: &DataValue) -> Result<Self, NvmError> {
                match value {
                    DataValue::U64(val) => Ok(*val as $t),
                    DataValue::I64(val) => Ok(*val as $t),
                    DataValue::F64(val) => Ok(*val as $t),
                    DataValue::Str(_) => Err(NvmError::DataValueExportFailed(
                        "Cannot convert string to scalar type.".to_string(),
                    )),
                }
            }
        }
    })*};
}

impl_try_from_data_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
