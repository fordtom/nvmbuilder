use crate::layout::{CrcLocation, LayoutError};

pub trait UnsignedInt: TryFrom<i64> + Copy + std::fmt::Display + Default
// where
//     <Self as TryFrom<i64>>::Error: std::fmt::Debug,
{
}

impl UnsignedInt for u8 {}
impl UnsignedInt for u16 {}
impl UnsignedInt for u32 {}
impl UnsignedInt for u64 {}

pub trait FromToml: Sized {
    fn from_toml_value(value: &toml::Value) -> Result<Self, LayoutError>;
}

pub fn extract_uint<T: UnsignedInt>(
    value: &toml::Value,
    error: LayoutError,
) -> Result<T, LayoutError> {
    match value {
        toml::Value::Integer(n) if *n >= 0 => T::try_from(*n).map_err(|_| error),
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

pub fn extract_crc_location<T: UnsignedInt>(
    value: &toml::Value,
    error: LayoutError,
) -> Result<CrcLocation<T>, LayoutError> {
    match value {
        toml::Value::String(s) => match s.as_str() {
            "start" => Ok(CrcLocation::Start),
            "end" => Ok(CrcLocation::End),
            _ => Err(error),
        },
        toml::Value::Integer(n) if *n >= 0 => {
            let addr = T::try_from(*n).map_err(|_| error)?;
            Ok(CrcLocation::Address(addr))
        }
        _ => Err(error),
    }
}
