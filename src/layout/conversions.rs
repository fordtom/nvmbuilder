use crate::layout::{
    LayoutError,
    types::{CrcLocation, DataValue},
};

pub trait FromToml: Sized {
    fn from_toml_value(value: &toml::Value) -> Result<Self, LayoutError>;
}

pub fn extract_uint(value: &toml::Value, error: LayoutError) -> Result<u32, LayoutError> {
    match value {
        toml::Value::Integer(n) if *n >= 0 => u32::try_from(*n).map_err(|_| error),
        _ => Err(error),
    }
}

pub fn extract_datavalue(
    value: &toml::Value,
    error: LayoutError,
) -> Result<DataValue, LayoutError> {
    match value {
        toml::Value::Integer(n) if *n >= 0 => {
            // Try to fit in smallest possible unsigned type
            if let Ok(val) = u8::try_from(*n) {
                Ok(DataValue::U8(val))
            } else if let Ok(val) = u16::try_from(*n) {
                Ok(DataValue::U16(val))
            } else if let Ok(val) = u32::try_from(*n) {
                Ok(DataValue::U32(val))
            } else if let Ok(val) = u64::try_from(*n) {
                Ok(DataValue::U64(val))
            } else {
                Err(error)
            }
        }
        toml::Value::Integer(n) => {
            // Handle negative integers - try signed types
            if let Ok(val) = i8::try_from(*n) {
                Ok(DataValue::I8(val))
            } else if let Ok(val) = i16::try_from(*n) {
                Ok(DataValue::I16(val))
            } else if let Ok(val) = i32::try_from(*n) {
                Ok(DataValue::I32(val))
            } else {
                Ok(DataValue::I64(*n as i64))
            }
        }
        toml::Value::Float(f) => {
            // Try f32 first, fall back to f64
            Ok(DataValue::F32(*f as f32))
        }
        _ => Err(error),
    }
}

pub fn extract_table(value: &toml::Value, error: LayoutError) -> Result<toml::Table, LayoutError> {
    match value {
        toml::Value::Table(table) => Ok(table.clone()),
        _ => Err(error),
    }
}

pub fn extract_string(value: &toml::Value, error: LayoutError) -> Result<String, LayoutError> {
    match value {
        toml::Value::String(s) => Ok(s.clone()),
        _ => Err(error),
    }
}

pub fn extract_crc_location(
    value: &toml::Value,
    error: LayoutError,
) -> Result<CrcLocation, LayoutError> {
    match value {
        toml::Value::String(s) => match s.as_str() {
            "start" => Ok(CrcLocation::Start),
            "end" => Ok(CrcLocation::End),
            _ => Err(error),
        },
        toml::Value::Integer(n) if *n >= 0 => {
            let addr = u32::try_from(*n).map_err(|_| error)?;
            Ok(CrcLocation::Address(addr))
        }
        _ => Err(error),
    }
}
