use super::entry::LeafEntry;
use super::errors::LayoutError;
use super::header::{CrcLocation, Header};
use super::settings::{Endianness, Settings};
use crate::variant::DataSheet;

use indexmap::IndexMap;
use serde::Deserialize;

/// Mutable state tracked during recursive bytestream building
struct BuildState {
    offset: usize,
    padding_count: u32,
}

/// Immutable configuration for bytestream building
pub struct BuildConfig<'a> {
    pub endianness: &'a Endianness,
    pub padding: u8,
    pub strict: bool,
}

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
        let mut state = BuildState {
            offset: 0,
            padding_count: 0,
        };
        let config = BuildConfig {
            endianness: &settings.endianness,
            padding: self.header.padding,
            strict,
        };

        Self::build_bytestream_inner(&self.data, data_sheet, &mut buffer, &mut state, &config)?;

        if matches!(self.header.crc_location, CrcLocation::Keyword(_)) {
            // Padding out to the 4 byte boundary for appended/prepended CRC32
            while !state.offset.is_multiple_of(4) {
                buffer.push(config.padding);
                state.offset += 1;
                state.padding_count += 1;
            }
        }

        Ok((buffer, state.padding_count))
    }

    fn build_bytestream_inner(
        table: &Entry,
        data_sheet: &DataSheet,
        buffer: &mut Vec<u8>,
        state: &mut BuildState,
        config: &BuildConfig,
    ) -> Result<(), LayoutError> {
        match table {
            Entry::Leaf(leaf) => {
                let alignment = leaf.get_alignment();
                while !state.offset.is_multiple_of(alignment) {
                    buffer.push(config.padding);
                    state.offset += 1;
                    state.padding_count += 1;
                }

                let bytes = leaf.emit_bytes(data_sheet, config)?;
                state.offset += bytes.len();
                buffer.extend(bytes);
            }
            Entry::Branch(branch) => {
                for (field_name, v) in branch.iter() {
                    Self::build_bytestream_inner(v, data_sheet, buffer, state, config).map_err(
                        |e| LayoutError::InField {
                            field: field_name.clone(),
                            source: Box::new(e),
                        },
                    )?;
                }
            }
        }
        Ok(())
    }
}
