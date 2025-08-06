mod config;
mod conversions;
mod types;

use crate::variants::DataSheet;
use config::{ConfigTable, ConfigValue, EntryType};
use std::fs;
use thiserror::Error;
use types::*;

macro_rules! extract {
    ($table:expr, $key:expr, $as:ident) => {
        $table
            .get($key)
            .ok_or(LayoutError::FailedToExtract($key.to_string()))?
            .$as()
            .ok_or(LayoutError::FailedToExtract($key.to_string()))?
    };
}

macro_rules! extract_owned {
    ($table:expr, $key:expr, $as:ident) => {{
        let value = $table
            .remove($key)
            .ok_or(LayoutError::FailedToExtract($key.to_string()))?;
        T::$as(value).ok_or(LayoutError::FailedToExtract($key.to_string()))?
    }};
}

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("File error: {0}")]
    FileError(String),

    #[error("Failed to extract {0}")]
    FailedToExtract(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Recursion failed: {0}")]
    RecursionFailed(String),

    #[error("Bytestream assembly failed: {0}")]
    BytestreamAssemblyFailed(String),
}

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
}

impl FlashBlock<toml::Table> {
    pub fn new(filename: &str, blockname: &str) -> Result<Self, LayoutError> {
        let file_content = fs::read_to_string(filename).map_err(|_| {
            LayoutError::FileError(("failed to open file: ".to_string() + filename).to_string())
        })?;
        let content: toml::Value = toml::from_str(&file_content).map_err(|_| {
            LayoutError::FileError(("failed to parse file: ".to_string() + filename).to_string())
        })?;
        Self::from_parsed_content(content, blockname)
    }
}

impl<T: ConfigTable> FlashBlock<T>
where
    T::Value: ConfigValue,
{
    fn from_parsed_content(content: T::Value, blockname: &str) -> Result<Self, LayoutError> {
        // Get root table
        let mut content = T::from_value(content).ok_or(LayoutError::FileError(
            "failed to extract root table.".to_string(),
        ))?;

        let block = content
            .remove(blockname)
            .ok_or(LayoutError::BlockNotFound(blockname.to_string()))?;
        let mut block_table =
            T::from_value(block).ok_or(LayoutError::BlockNotFound(blockname.to_string()))?;

        let data = extract_owned!(block_table, "data", from_value);
        let header = extract!(block_table, "header", as_table);

        let settings = extract!(content, "settings", as_table);
        let crc_settings = extract!(settings, "crc", as_table);

        // Extract CRC data
        let crc_polynomial = extract!(crc_settings, "polynomial", as_integer);
        let crc_start_value = extract!(crc_settings, "start", as_integer);
        let crc_xor_out = extract!(crc_settings, "xor_out", as_integer);
        let crc_reflect = extract!(crc_settings, "reverse", as_bool);
        let crc_location = extract!(header, "crc_location", as_integer);

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
        let start_address = extract!(header, "start_address", as_integer);
        let length = extract!(header, "length", as_integer);
        let padding = extract!(header, "padding", as_integer);

        let endianness = match extract!(settings, "endianness", as_string) {
            "little" => Endianness::Little,
            "big" => Endianness::Big,
            _ => return Err(LayoutError::FailedToExtract("endianness".to_string())),
        };

        Ok(Self {
            start_address: start_address as u32,
            length: length as u32,
            padding: DataValue::U8(padding as u8),
            endianness,
            crc_data,
            data,
        })
    }

    pub fn build_bytestream(&self, data_sheet: &DataSheet) -> Result<Vec<u8>, LayoutError> {
        let mut buffer = Vec::with_capacity(self.length as usize);
        let mut offset = 0u32;
        Self::build_bytestream_inner(
            &self.data,
            data_sheet,
            &mut buffer,
            &mut offset,
            &self.endianness,
            &self.padding,
        )?;
        Ok(buffer)
    }

    fn build_bytestream_inner(
        table: &dyn ConfigTable<Value = T::Value>,
        data_sheet: &DataSheet,
        buffer: &mut Vec<u8>,
        offset: &mut u32,
        endianness: &Endianness,
        padding: &DataValue,
    ) -> Result<(), LayoutError> {
        for (_, v) in table.iter() {
            match v.classify_entry() {
                Ok(EntryType::DataEntry {
                    type_str,
                    config_value,
                }) => {}
                Ok(EntryType::NameEntry { type_str, name }) => {
                    let data = data_sheet
                        .retrieve_cell_data(&name)
                        .map_err(|_| LayoutError::FailedToExtract(name.to_string()))?;
                }
                Ok(EntryType::NestedTable(nested_table)) => {
                    Self::build_bytestream_inner(
                        nested_table,
                        data_sheet,
                        buffer,
                        offset,
                        endianness,
                        padding,
                    )?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
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
