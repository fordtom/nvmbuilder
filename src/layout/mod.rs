mod conversions;

use conversions::{UnsignedInt, extract_crc_location, extract_table, extract_uint};
use std::fs;

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
}

#[derive(Debug, Clone)]
pub enum CrcLocation<AddrSize> {
    Start,
    End,
    Address(AddrSize),
}

pub struct FlashBlock<AddrSize, Smallest> {
    // From the block header
    start_address: AddrSize,
    length: AddrSize,
    padding: Smallest,

    // Data alignment
    alignment: toml::Table,

    // Options and value for the CRC32
    crc_polynomial: u32,
    crc_location: CrcLocation<AddrSize>,
    crc32: Option<u32>,

    // The data itself
    data: toml::Table,

    // The output
    bytestream: Option<Vec<Smallest>>,
}

impl<AddrSize: UnsignedInt, Smallest: UnsignedInt> FlashBlock<AddrSize, Smallest> {
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

        // Get the alignment info
        let alignment = settings
            .get("alignment")
            .ok_or(LayoutError::InvalidSettings)?;

        Ok(Self {
            start_address: extract_uint(start_address, LayoutError::InvalidHeader)?,
            length: extract_uint(length, LayoutError::InvalidHeader)?,
            padding: extract_uint(padding, LayoutError::NoPadding)?,
            alignment: extract_table(alignment, LayoutError::InvalidSettings)?,
            crc_polynomial: extract_uint(crc_polynomial, LayoutError::InvalidSettings)?,
            crc_location: extract_crc_location(crc_location, LayoutError::InvalidHeader)?,
            crc32: None,
            data: extract_table(data, LayoutError::InvalidData)?,
            bytestream: None,
        })
    }

    // Getter methods
    pub fn start_address(&self) -> &AddrSize {
        &self.start_address
    }

    pub fn length(&self) -> &AddrSize {
        &self.length
    }

    pub fn padding(&self) -> &Smallest {
        &self.padding
    }

    pub fn crc_poly(&self) -> u32 {
        self.crc_polynomial
    }

    pub fn crc_location(&self) -> &CrcLocation<AddrSize> {
        &self.crc_location
    }

    pub fn data(&self) -> &toml::Table {
        &self.data
    }
}
