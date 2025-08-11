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
        let polynomial = u32::try_from(extract!(crc_settings, "polynomial", as_integer))
            .map_err(|_| NvmError::FailedToExtract("polynomial".to_string()))?;
        let start_value = u32::try_from(extract!(crc_settings, "start", as_integer))
            .map_err(|_| NvmError::FailedToExtract("start".to_string()))?;
        let xor_out = u32::try_from(extract!(crc_settings, "xor_out", as_integer))
            .map_err(|_| NvmError::FailedToExtract("xor_out".to_string()))?;
        let reflect = extract!(crc_settings, "reverse", as_bool);
        let location = u32::try_from(extract!(header, "crc_location", as_integer))
            .map_err(|_| NvmError::FailedToExtract("crc_location".to_string()))?;

        // Pack CRC data
        let crc_data = CrcData {
            polynomial,
            start_value,
            xor_out,
            reflect,
            location,
            value: None,
        };

        // Extract header data
        let start_address = u32::try_from(extract!(header, "start_address", as_integer))
            .map_err(|_| NvmError::FailedToExtract("start_address".to_string()))?;
        let length = u32::try_from(extract!(header, "length", as_integer))
            .map_err(|_| NvmError::FailedToExtract("length".to_string()))?;
        let padding = u8::try_from(extract!(header, "padding", as_integer))
            .map_err(|_| NvmError::FailedToExtract("padding".to_string()))?;

        let endianness = match extract!(settings, "endianness", as_string) {
            "little" => Endianness::Little,
            "big" => Endianness::Big,
            _ => return Err(NvmError::FailedToExtract("endianness".to_string())),
        };

        Ok(Self {
            start_address,
            length,
            padding: DataValue::U8(padding),
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
                    let data_value = match source {
                        EntrySource::Value(value) => value.export_datavalue(&type_str)?,
                        EntrySource::Name(name) => {
                            data_sheet.retrieve_cell_data(&name, &type_str)?
                        }
                    };
                    buffer.extend(data_value.to_bytes(endianness));
                    *offset += data_value.size_bytes();
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
