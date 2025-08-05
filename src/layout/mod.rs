mod config;
mod conversions;
mod types;

use config::{ConfigTable, ConfigValue};
use std::fs;
use types::*;

macro_rules! extract {
    ($table:expr, $key:expr, $as:ident, $error:expr) => {
        $table.get($key).ok_or($error)?.$as().ok_or($error)?
    };
}

macro_rules! extract_owned {
    ($table:expr, $key:expr, $as:ident, $error:expr) => {{
        let value = $table.remove($key).ok_or($error)?;
        T::$as(value).ok_or($error)?
    }};
}

#[derive(Debug)]
pub enum LayoutError {
    FailedToReadFile,
    FailedToParseFile,
    SettingsNotFound,
    InvalidSettings,
    BlockNotFound,
    NoPadding,
    InvalidHeader,
    InvalidData,
    InvalidCell,
    InvalidUnitSize,
    BadDataValueExtraction,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LayoutError {}

#[derive(Debug)]
pub struct CrcData {
    polynomial: u32,
    start_value: u32,
    xor_out: u32,
    reflect: bool,
    location: u32,
    value: Option<u32>,
}

pub struct FlashBlock<T: ConfigTable> {
    // From the block header
    start_address: u32,
    length: u32,
    padding: DataValue,
    endianness: Endianness,

    // CRC32 data
    crc_data: CrcData,

    // The data itself
    pub data: T,

    // The output
    pub bytestream: Option<Vec<u8>>,
}

impl FlashBlock<toml::Table> {
    pub fn new(contents: &str, blockname: &str) -> Result<Self, LayoutError> {
        let content: toml::Value =
            toml::from_str(contents).map_err(|_| LayoutError::FailedToParseFile)?;
        Self::from_parsed_content(content, blockname)
    }
}

impl<T: ConfigTable> FlashBlock<T>
where
    T::Value: ConfigValue,
{
    fn from_parsed_content(content: T::Value, blockname: &str) -> Result<Self, LayoutError> {
        // Get root table
        let mut content = T::from_value(content).ok_or(LayoutError::InvalidData)?;

        let block = content
            .remove(blockname)
            .ok_or(LayoutError::BlockNotFound)?;
        let mut block_table = T::from_value(block).ok_or(LayoutError::BlockNotFound)?;

        let data = extract_owned!(block_table, "data", from_value, LayoutError::InvalidData);
        let header = extract!(block_table, "header", as_table, LayoutError::InvalidHeader);

        let settings = extract!(content, "settings", as_table, LayoutError::SettingsNotFound);
        let crc_settings = extract!(settings, "crc", as_table, LayoutError::InvalidSettings);

        // Extract CRC data
        let crc_polynomial = extract!(
            crc_settings,
            "polynomial",
            as_integer,
            LayoutError::InvalidSettings
        );
        let crc_start_value = extract!(
            crc_settings,
            "start",
            as_integer,
            LayoutError::InvalidSettings
        );
        let crc_xor_out = extract!(
            crc_settings,
            "xor_out",
            as_integer,
            LayoutError::InvalidSettings
        );
        let crc_reflect = extract!(
            crc_settings,
            "reverse",
            as_bool,
            LayoutError::InvalidSettings
        );
        let crc_location = extract!(
            header,
            "crc_location",
            as_integer,
            LayoutError::InvalidSettings
        );

        // Pack CRC data
        let crc_data = CrcData {
            polynomial: crc_polynomial as u32,
            start_value: crc_start_value as u32,
            xor_out: crc_xor_out as u32,
            reflect: crc_reflect,
            location: crc_location as u32,
            value: None,
        };

        // Extract header data
        let start_address = extract!(
            header,
            "start_address",
            as_integer,
            LayoutError::InvalidHeader
        );
        let length = extract!(header, "length", as_integer, LayoutError::InvalidHeader);
        let padding = extract!(header, "padding", as_integer, LayoutError::NoPadding);

        let endianness = match extract!(
            settings,
            "endianness",
            as_string,
            LayoutError::InvalidHeader
        ) {
            "little" => Endianness::Little,
            "big" => Endianness::Big,
            _ => return Err(LayoutError::InvalidHeader),
        };

        Ok(Self {
            start_address: start_address as u32,
            length: length as u32,
            padding: DataValue::U8(padding as u8),
            endianness,
            crc_data,
            data,
            bytestream: None,
        })
    }

    pub fn value_to_bytes(&self, value: &DataValue) -> Vec<u8> {
        value.to_bytes(self.endianness)
    }

    // Getter methods
    pub fn start_address(&self) -> &u32 {
        &self.start_address
    }

    pub fn length(&self) -> &u32 {
        &self.length
    }

    pub fn padding(&self) -> &DataValue {
        &self.padding
    }

    pub fn crc_poly(&self) -> u32 {
        self.crc_data.polynomial
    }

    pub fn crc_location(&self) -> &u32 {
        &self.crc_data.location
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }
}
