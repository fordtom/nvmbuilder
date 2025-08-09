use crate::error::*;
use crate::types::*;
use crate::variants::DataSheet;
use std::fs;

macro_rules! extract {
    ($table:expr, $key:expr, $as:ident) => {
        $table
            .get($key)
            .ok_or(NvmError::FailedToExtract($key.to_string()))?
            .$as()
            .ok_or(NvmError::FailedToExtract($key.to_string()))?
    };
}

macro_rules! extract_owned {
    ($table:expr, $key:expr, $as:ident) => {{
        let value = $table
            .remove($key)
            .ok_or(NvmError::FailedToExtract($key.to_string()))?;
        T::$as(value).ok_or(NvmError::FailedToExtract($key.to_string()))?
    }};
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
    pub fn new(filename: &str, blockname: &str) -> Result<Self, NvmError> {
        let file_content = fs::read_to_string(filename)
            .map_err(|_| NvmError::FileError("failed to open file: ".to_string() + filename))?;
        let content: toml::Value = toml::from_str(&file_content)
            .map_err(|_| NvmError::FileError("failed to parse file: ".to_string() + filename))?;
        Self::from_parsed_content(content, blockname)
    }
}

impl<T: ConfigTable> FlashBlock<T>
where
    T::Value: ConfigValue,
{
    fn from_parsed_content(content: T::Value, blockname: &str) -> Result<Self, NvmError> {
        // Get root table
        let mut content = T::from_value(content).ok_or(NvmError::FileError(
            "failed to extract root table.".to_string(),
        ))?;

        let block = content
            .remove(blockname)
            .ok_or(NvmError::BlockNotFound(blockname.to_string()))?;
        let mut block_table =
            T::from_value(block).ok_or(NvmError::BlockNotFound(blockname.to_string()))?;

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
            _ => return Err(NvmError::FailedToExtract("endianness".to_string())),
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

    pub fn build_bytestream(&self, data_sheet: &DataSheet) -> Result<Vec<u8>, NvmError> {
        let mut buffer = Vec::with_capacity(self.length as usize);
        let mut offset = 0;
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
        offset: &mut usize,
        endianness: &Endianness,
        padding: &DataValue,
    ) -> Result<(), NvmError> {
        for (_, v) in table.iter() {
            match v.classify_entry() {
                // Handle single value
                Ok(EntryType::SingleEntry { type_str, source }) => {
                    match source {
                        EntrySource::Value(value) => {
                            let value = value.export_datavalue(&type_str)?;
                            buffer.extend(value.to_bytes(endianness));
                            *offset += value.size_bytes();
                        }
                        EntrySource::Name(name) => {
                            let value = data_sheet
                                .retrieve_cell_data(&name)
                                .map_err(|_| NvmError::FailedToExtract(name.to_string()))?;

                            // Either retrieve cell data takes types, or handle it here
                            // let value = value.export_datavalue(&type_str)?;
                            // buffer.extend(value.to_bytes(endianness));
                            // *offset += value.size_bytes();
                        }
                    }
                }

                // Handle string (TODO)

                // Handle array (TODO)

                // If we have a nested table we recurse
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

                // Pass up errors
                Err(e) => {
                    return Err(e);
                }

                // temporary before we implement strings/arrays
                _ => {
                    return Err(NvmError::FailedToExtract(
                        "unsupported entry type.".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}
