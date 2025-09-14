use super::entry::ScalarType;
use super::settings::EndianBytes;
use super::settings::Endianness;
use crate::error::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueSource {
    Single(DataValue),
    Array(Vec<DataValue>),
}

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

    pub fn string_to_bytes(&self) -> Result<Vec<u8>, NvmError> {
        match self {
            DataValue::Str(val) => Ok(val.as_bytes().to_vec()),
            _ => Err(NvmError::DataValueExportFailed(
                "String expected for string type.".to_string(),
            )),
        }
    }
}
