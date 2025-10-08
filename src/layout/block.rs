use super::entry::LeafEntry;
use super::errors::LayoutError;
use super::header::{CrcLocation, Header};
use super::settings::{Endianness, Settings};
use crate::variant::DataSheet;

use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(flatten)]
    pub blocks: IndexMap<String, Block>,
}

/// Flash block.
#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: Header,
    pub data: Entry,
}

/// Any entry - should always be either a leaf or a branch (more entries).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Leaf(LeafEntry),
    Branch(IndexMap<String, Entry>),
}

impl Block {
    pub fn build_bytestream(
        &self,
        data_sheet: &DataSheet,
        settings: &Settings,
        strict: bool,
    ) -> Result<(Vec<u8>, u32), LayoutError> {
        let mut buffer = Vec::with_capacity(self.header.length as usize);
        let mut offset = 0;
        let mut padding_count = 0;

        Self::build_bytestream_inner(
            &self.data,
            data_sheet,
            &mut buffer,
            &mut offset,
            &settings.endianness,
            &self.header.padding,
            strict,
            &mut padding_count,
        )?;

        if matches!(self.header.crc_location, CrcLocation::Keyword(_)) {
            // Padding out to the 4 byte boundary for appended/prepended CRC32
            while offset % 4 != 0 {
                buffer.push(self.header.padding);
                offset += 1;
                padding_count += 1;
            }
        }

        Ok((buffer, padding_count))
    }

    fn build_bytestream_inner(
        table: &Entry,
        data_sheet: &DataSheet,
        buffer: &mut Vec<u8>,
        offset: &mut usize,
        endianness: &Endianness,
        padding: &u8,
        strict: bool,
        padding_count: &mut u32,
    ) -> Result<(), LayoutError> {
        match table {
            Entry::Leaf(leaf) => {
                let alignment = leaf.get_alignment();
                while *offset % alignment != 0 {
                    buffer.push(*padding);
                    *offset += 1;
                    *padding_count += 1;
                }

                let bytes = leaf.emit_bytes(data_sheet, endianness, padding, strict)?;
                *offset += bytes.len();
                buffer.extend(bytes);
            }
            Entry::Branch(branch) => {
                for (field_name, v) in branch.iter() {
                    Self::build_bytestream_inner(
                        v,
                        data_sheet,
                        buffer,
                        offset,
                        endianness,
                        padding,
                        strict,
                        padding_count,
                    )
                    .map_err(|e| LayoutError::InField {
                        field: field_name.clone(),
                        source: Box::new(e),
                    })?;
                }
            }
        }
        Ok(())
    }
}
