#![allow(dead_code, unused_variables)]

use crate::layout::{
    LayoutError,
    types::{CrcLocation, DataValue, TypeSpec},
};

pub fn extract_uint(value: &toml::Value, error: LayoutError) -> Result<u32, LayoutError> {
    match value {
        toml::Value::Integer(n) if *n >= 0 => u32::try_from(*n).map_err(|_| error),
        _ => Err(error),
    }
}

pub fn extract_datavalue(value: &toml::Value) -> Result<Vec<DataValue>, LayoutError> {
    match value {
        toml::Value::Table(table) => {
            let type_value = table
                .get("type")
                .ok_or(LayoutError::BadDataValueExtraction)?;
            let typespec = TypeSpec::from_value(type_value)?;
            typespec.extract_datavalues(value)
        }
        _ => Err(LayoutError::BadDataValueExtraction),
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
