mod conversions;
mod types;

use conversions::{extract_crc_location, extract_datavalue, extract_table, extract_uint};
use std::fs;
use types::*;

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

pub struct FlashBlock {
    // From the block header
    start_address: u32,
    length: u32,
    padding: DataValue,

    // Data alignment
    address_width: AddressWidth,
    endianness: Endianness,
    unit_size: MemoryUnitSize,

    // Options and value for the CRC32
    crc_polynomial: u32,
    crc_location: CrcLocation,
    crc32: Option<u32>,

    // The data itself
    data: toml::Table,

    // The output
    bytestream: Option<Vec<u8>>,
}

impl FlashBlock {
    pub fn new(filename: String, blockname: String) -> Result<Self, LayoutError> {
        // Open and parse the file
        let file: String =
            fs::read_to_string(filename).map_err(|_| LayoutError::FailedToReadFile)?;
        let content: toml::Value =
            toml::from_str(&file).map_err(|_| LayoutError::FailedToParseFile)?;

        // Get the settings and the named block
        let block = content.get(blockname).ok_or(LayoutError::BlockNotFound)?;
        let settings = content
            .get("settings")
            .ok_or(LayoutError::SettingsNotFound)?;

        // Get the data and the header from the block
        let header = block.get("header").ok_or(LayoutError::InvalidHeader)?;
        let data = block.get("data").ok_or(LayoutError::InvalidData)?;

        // Get the header info
        let start_address = header
            .get("start_address")
            .ok_or(LayoutError::InvalidHeader)?;
        let length = header.get("length").ok_or(LayoutError::InvalidHeader)?;

        // Get padding from header or settings fallback
        let padding = header
            .get("padding")
            .or_else(|| settings.get("padding"))
            .ok_or(LayoutError::NoPadding)?;

        // Get the crc info
        let crc_polynomial = settings
            .get("crc_polynomial")
            .ok_or(LayoutError::InvalidSettings)?;
        let crc_location = header
            .get("crc_location")
            .ok_or(LayoutError::InvalidHeader)?;

        // Get the unit size
        let unit_size = settings
            .get("unit_size")
            .ok_or(LayoutError::InvalidSettings)?;
        let unit_size = extract_uint(unit_size, LayoutError::InvalidSettings)? as usize;
        let unit_size = MemoryUnitSize::from_bytes(unit_size)?;

        Ok(Self {
            start_address: extract_uint(start_address, LayoutError::InvalidHeader)?,
            length: extract_uint(length, LayoutError::InvalidHeader)?,
            padding: extract_datavalue(padding)?[0].clone(),
            address_width: AddressWidth::Bits32,
            unit_size,
            endianness: Endianness::Little,
            crc_polynomial: extract_uint(crc_polynomial, LayoutError::InvalidSettings)?,
            crc_location: extract_crc_location(crc_location, LayoutError::InvalidHeader)?,
            crc32: None,
            data: extract_table(data, LayoutError::InvalidData)?,
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
        self.crc_polynomial
    }

    pub fn crc_location(&self) -> &CrcLocation {
        &self.crc_location
    }

    pub fn data(&self) -> &toml::Table {
        &self.data
    }
}
