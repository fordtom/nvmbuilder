use crate::error::*;
use indexmap::IndexMap;
use serde::{de, Deserialize, Deserializer};
use std::fmt;

/// Top level struct that contains the settings and the block.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(
        default = "default_offset",
        deserialize_with = "deserialize_u32_from_str_or_int"
    )]
    pub offset: u32,
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

fn default_offset() -> u32 {
    0
}

fn parse_u32_from_str(offset: &str) -> Result<u32, ()> {
    let s = offset.trim();
    let (radix, digits) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, hex)
    } else {
        (10, s)
    };

    u32::from_str_radix(&digits.replace('_', ""), radix).map_err(|_| ())
}

pub fn deserialize_u32_from_str_or_int<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    struct U32Visitor;

    impl<'de> de::Visitor<'de> for U32Visitor {
        type Value = u32;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a u32 or a string containing a hex (0x...) or decimal number")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(v).map_err(|_| E::custom("Offset value out of range for u32."))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v < 0 {
                return Err(E::custom("Offset must be a non-negative number."));
            }
            u32::try_from(v as u64).map_err(|_| E::custom("Offset value out of range for u32."))
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            parse_u32_from_str(s).map_err(|_| {
                E::custom(format!(
                    "Invalid offset value '{}' in layout file. Must be valid hexadecimal or decimal address.",
                    s
                ))
            })
        }

        fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&s)
        }
    }

    deserializer.deserialize_any(U32Visitor)
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

macro_rules! impl_try_from_data_value {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<&DataValue> for $t {
            type Error = NvmError;
            fn try_from(value: &DataValue) -> Result<Self, NvmError> {
                match value {
                    DataValue::U64(val) => Ok(*val as $t),
                    DataValue::I64(val) => Ok(*val as $t),
                    DataValue::F64(val) => Ok(*val as $t),
                    DataValue::Str(_) => {
                        return Err(NvmError::DataValueExportFailed(
                            "Cannot convert string to scalar type.".to_string(),
                        ));
                    }
                }
            }
        }
    )* }; }

impl_try_from_data_value!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_toml_with_offset(offset_line: &str) -> String {
        format!(
            r#"{offset}
[settings]
endianness = "little"

[settings.crc]
polynomial = 0x04C11DB7
start = 0xFFFFFFFF
xor_out = 0xFFFFFFFF
ref_in = true
ref_out = true

[block.header]
start_address = 0x1000
length = 0x20
crc_location = "end"

[block.data]
item = {{ value = 1, type = "u8" }}
"#,
            offset = offset_line
        )
    }

    #[test]
    fn parses_offset_hex_and_decimal() {
        let toml_hex = minimal_toml_with_offset("offset = 0x8000");
        let cfg_hex: Config = toml::from_str(&toml_hex).expect("parse hex offset");
        assert_eq!(cfg_hex.offset, 0x8000);

        let toml_dec = minimal_toml_with_offset("offset = 4096");
        let cfg_dec: Config = toml::from_str(&toml_dec).expect("parse dec offset");
        assert_eq!(cfg_dec.offset, 4096);
    }

    #[test]
    fn missing_offset_defaults_to_zero() {
        let toml_no = minimal_toml_with_offset("");
        let cfg: Config = toml::from_str(&toml_no).expect("parse without offset");
        assert_eq!(cfg.offset, 0);
    }

    #[test]
    fn invalid_offset_reports_helpful_error() {
        let toml_bad = minimal_toml_with_offset("offset = '0xGGGG'");
        let err = toml::from_str::<Config>(&toml_bad).unwrap_err().to_string();
        assert!(err.contains("Invalid offset value"));
    }
}
