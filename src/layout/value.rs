use super::conversions::convert_value_to_bytes;
use super::entry::ScalarType;
use super::errors::LayoutError;
use super::settings::Endianness;
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
        strict: bool,
    ) -> Result<Vec<u8>, LayoutError> {
        convert_value_to_bytes(self, scalar_type, endianness, strict)
    }

    pub fn string_to_bytes(&self) -> Result<Vec<u8>, LayoutError> {
        match self {
            DataValue::Str(val) => Ok(val.as_bytes().to_vec()),
            _ => Err(LayoutError::DataValueExportFailed(
                "String expected for string type.".to_string(),
            )),
        }
    }
}
