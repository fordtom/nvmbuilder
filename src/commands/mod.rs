pub mod generate;

use crate::args::Args;
use crate::error::NvmError;
use crate::variant::DataSheet;
use rayon::prelude::*;
use crate::layout;
use crate::output::args::OutputFormat;
use crate::writer::write_output;

#[derive(Debug, Clone)]
pub struct CrcRecord {
    pub address: u32,
    pub value: u32,
    pub source_range: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct ProcessedBlock {
    pub name: String,
    pub address_range: (u32, u32),
    pub data: Vec<u8>,
    pub crc_records: Vec<CrcRecord>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct BlockMetadata {}

#[derive(Debug, Clone)]
pub struct CombinedCrcSpec {
    pub source_ranges: Vec<(u32, u32)>,
    pub dest_address: u32,
}

#[derive(Debug, Clone)]
pub struct CombinedOutputConfig {
    pub record_width: usize,
    pub format: OutputFormat,
    pub combined_crc: Option<CombinedCrcSpec>,
}

#[derive(Debug, Clone)]
pub struct CombinedBlock {
    pub hex: String,
    pub processed: Vec<ProcessedBlock>,
}

pub fn build_separate_blocks(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    args.layout
        .blocks
        .par_iter()
        .try_for_each(|input| generate::build_block_single(input, data_sheet, args))
}

pub fn build_single_file(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    if args.layout.blocks.is_empty() {
        return Err(NvmError::MiscError("no blocks provided".to_string()));
    }

    // Load first layout; ensure all blocks share compatible settings
    let first = &args.layout.blocks[0];
    let first_layout = layout::load_layout(&first.file)?;
    crate::output::checksum::init_crc_algorithm(&first_layout.settings.crc);

    for b in &args.layout.blocks[1..] {
        let layout_i = layout::load_layout(&b.file)?;
        if !settings_equal(&first_layout.settings, &layout_i.settings) {
            return Err(NvmError::MiscError(
                "All blocks must share identical layout settings for combined output".to_string(),
            ));
        }
    }

    // Build raw block data
    let mut blocks_data: Vec<BlockData> = Vec::with_capacity(args.layout.blocks.len());
    for bn in &args.layout.blocks {
        let cfg = layout::load_layout(&bn.file)?;
        let block = cfg
            .blocks
            .get(&bn.name)
            .ok_or(NvmError::BlockNotFound(bn.name.clone()))?;
        let mut bytestream = block.build_bytestream(
            data_sheet,
            &cfg.settings,
            args.layout.strict,
        )?;
        blocks_data.push(BlockData {
            name: bn.name.clone(),
            header: block.header.clone(),
            data: bytestream.drain(..).collect(),
        });
    }

    let combined_cfg = CombinedOutputConfig {
        record_width: args.output.record_width as usize,
        format: args.output.format,
        combined_crc: None,
    };

    let combined = build_combined_blocks(blocks_data, &first_layout, &combined_cfg)?;
    write_output(&args.output, "combined", &combined.hex)
}

fn settings_equal(a: &layout::settings::Settings, b: &layout::settings::Settings) -> bool {
    use layout::settings::{CrcData, Endianness};
    fn crc_eq(x: &CrcData, y: &CrcData) -> bool {
        x.polynomial == y.polynomial
            && x.start == y.start
            && x.xor_out == y.xor_out
            && x.ref_in == y.ref_in
            && x.ref_out == y.ref_out
    }
    a.endianness as u8 == b.endianness as u8
        && a.virtual_offset == b.virtual_offset
        && a.byte_swap == b.byte_swap
        && a.pad_to_end == b.pad_to_end
        && crc_eq(&a.crc, &b.crc)
}

pub fn build_combined_blocks(
    blocks: Vec<BlockData>,
    layout_cfg: &layout::block::Config,
    output_config: &CombinedOutputConfig,
) -> Result<CombinedBlock, NvmError> {
    let processed = prepare_combined_data(blocks, layout_cfg, output_config)?;
    let hex = emit_combined_hex(&processed, layout_cfg, output_config)?;
    Ok(CombinedBlock { hex, processed })
}

#[derive(Debug, Clone)]
pub struct BlockData {
    pub name: String,
    pub header: layout::header::Header,
    pub data: Vec<u8>,
}

pub fn prepare_combined_data(
    blocks: Vec<BlockData>,
    layout_cfg: &layout::block::Config,
    output_config: &CombinedOutputConfig,
) -> Result<Vec<ProcessedBlock>, NvmError> {
    use crate::output::checksum;
    use crate::layout::settings::{Endianness, Settings};

    let settings: &Settings = &layout_cfg.settings;

    let mut processed: Vec<ProcessedBlock> = Vec::with_capacity(blocks.len());

    for b in blocks.into_iter() {
        let mut data = b.data;

        // Optional byte swap before CRC computation
        if layout_cfg.settings.byte_swap {
            for chunk in data.chunks_exact_mut(2) {
                chunk.swap(0, 1);
            }
        }

        // Compute CRC over payload
        let crc_val = checksum::calculate_crc(&data);
        let mut crc_bytes = match settings.endianness {
            Endianness::Big => crc_val.to_be_bytes(),
            Endianness::Little => crc_val.to_le_bytes(),
        };
        if layout_cfg.settings.byte_swap {
            for chunk in crc_bytes.chunks_exact_mut(2) {
                chunk.swap(0, 1);
            }
        }

        // Place CRC either at an absolute address or append at end
        let mut crc_records: Vec<CrcRecord> = Vec::new();
        let mut out = data.clone();
        let start = b.header.start_address;
        let mut end_address = start + out.len() as u32;
        match &b.header.crc_location {
            layout::header::CrcLocation::Address(addr) => {
                let offset = addr.checked_sub(start).ok_or_else(|| {
                    NvmError::HexOutputError("CRC address before block start.".to_string())
                })? as usize;
                if offset < out.len() {
                    return Err(NvmError::HexOutputError(
                        "CRC overlaps with payload.".to_string(),
                    ));
                }
                let needed = offset + 4;
                if needed > b.header.length as usize {
                    return Err(NvmError::HexOutputError(
                        "CRC location would overrun block.".to_string(),
                    ));
                }
                if out.len() < needed {
                    out.resize(needed, b.header.padding);
                }
                out[offset..offset + 4].copy_from_slice(&crc_bytes);
                end_address = start + (needed as u32);
                crc_records.push(CrcRecord {
                    address: *addr,
                    value: crc_val,
                    source_range: (start, start + data.len() as u32),
                });
            }
            layout::header::CrcLocation::Keyword(s) if s == "end" => {
                // Align to 4 bytes then append
                while out.len() % 4 != 0 {
                    out.push(b.header.padding);
                }
                let crc_addr = start + out.len() as u32;
                out.extend_from_slice(&crc_bytes);
                end_address = start + out.len() as u32;
                if settings.pad_to_end {
                    if out.len() < b.header.length as usize {
                        out.resize(b.header.length as usize, b.header.padding);
                        end_address = start + b.header.length;
                    }
                }
                crc_records.push(CrcRecord {
                    address: crc_addr,
                    value: crc_val,
                    source_range: (start, start + data.len() as u32),
                });
            }
            _ => {
                return Err(NvmError::HexOutputError(
                    "Invalid CRC location keyword".to_string(),
                ));
            }
        }

        processed.push(ProcessedBlock {
            name: b.name,
            address_range: (start + settings.virtual_offset, end_address + settings.virtual_offset),
            data: out,
            crc_records,
            metadata: BlockMetadata::default(),
        });
    }

    // Optional cross-block CRC over specified address ranges
    if let Some(spec) = &output_config.combined_crc {
        use std::collections::BTreeMap;

        // Aggregate bytes by absolute physical address; later inserts overwrite earlier
        let mut address_to_byte: BTreeMap<u32, u8> = BTreeMap::new();
        for &(range_start, range_end) in &spec.source_ranges {
            let start_addr = range_start.min(range_end);
            let end_addr = range_start.max(range_end);

            for p in &processed {
                // Convert processed block range back to physical (remove virtual offset)
                let phys_start = p
                    .address_range
                    .0
                    .saturating_sub(settings.virtual_offset);
                let phys_end = p
                    .address_range
                    .1
                    .saturating_sub(settings.virtual_offset);

                // Compute overlap
                let overlap_start = phys_start.max(start_addr);
                let overlap_end = phys_end.min(end_addr);
                if overlap_end > overlap_start {
                    let offset = (overlap_start - phys_start) as usize;
                    let len = (overlap_end - overlap_start) as usize;
                    let slice = &p.data[offset..offset + len];
                    for (i, &byte) in slice.iter().enumerate() {
                        address_to_byte.insert(overlap_start + i as u32, byte);
                    }
                }
            }
        }

        // Build concatenated byte stream in ascending address order
        let mut concatenated: Vec<u8> = Vec::with_capacity(address_to_byte.len());
        for (_, byte) in address_to_byte.iter() {
            concatenated.push(*byte);
        }

        // Compute CRC
        let crc_val = checksum::calculate_crc(&concatenated);
        let mut crc_bytes = match settings.endianness {
            Endianness::Big => crc_val.to_be_bytes(),
            Endianness::Little => crc_val.to_le_bytes(),
        };
        if layout_cfg.settings.byte_swap {
            for chunk in crc_bytes.chunks_exact_mut(2) {
                chunk.swap(0, 1);
            }
        }

        // Create a synthetic block containing only the CRC at the destination address
        let virt_dest = spec.dest_address + settings.virtual_offset;
        processed.push(ProcessedBlock {
            name: "combined_crc".to_string(),
            address_range: (virt_dest, virt_dest + 4),
            data: crc_bytes.to_vec(),
            crc_records: vec![CrcRecord {
                address: spec.dest_address,
                value: crc_val,
                source_range: (*spec
                    .source_ranges
                    .iter()
                    .map(|(s, _)| s)
                    .min()
                    .unwrap_or(&0), *spec
                    .source_ranges
                    .iter()
                    .map(|(_, e)| e)
                    .max()
                    .unwrap_or(&0)),
            }],
            metadata: BlockMetadata::default(),
        });
    }

    Ok(processed)
}

pub fn emit_combined_hex(
    processed: &[ProcessedBlock],
    _layout_cfg: &layout::block::Config,
    output_config: &CombinedOutputConfig,
) -> Result<String, NvmError> {
    use crate::output::{emit_hex, DataRange};

    let mut ranges: Vec<DataRange> = Vec::with_capacity(processed.len());
    for p in processed {
        ranges.push(DataRange {
            start_address: p.address_range.0,
            bytestream: &p.data,
        });
    }

    emit_hex(&ranges, output_config.record_width, output_config.format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::header::{CrcLocation, Header};
    use crate::layout::settings::{CrcData, Endianness, Settings};
    use indexmap::IndexMap;

    fn sample_settings() -> Settings {
        Settings {
            endianness: Endianness::Little,
            virtual_offset: 0,
            byte_swap: false,
            pad_to_end: false,
            crc: CrcData {
                polynomial: 0x04C11DB7,
                start: 0xFFFF_FFFF,
                xor_out: 0xFFFF_FFFF,
                ref_in: true,
                ref_out: true,
            },
        }
    }

    fn sample_config() -> layout::block::Config {
        let settings = sample_settings();
        crate::output::checksum::init_crc_algorithm(&settings.crc);
        layout::block::Config {
            settings,
            blocks: IndexMap::new(),
        }
    }

    #[test]
    fn prepare_appends_crc_at_end_keyword() {
        let layout_cfg = sample_config();

        let header = Header {
            start_address: 0x1000,
            length: 32,
            crc_location: CrcLocation::Keyword("end".to_string()),
            padding: 0xFF,
        };

        let block = BlockData {
            name: "blk".to_string(),
            header,
            data: vec![1u8, 2, 3, 4],
        };
        let output_cfg = CombinedOutputConfig {
            record_width: 16,
            format: crate::output::args::OutputFormat::Hex,
            combined_crc: None,
        };
        let processed = prepare_combined_data(vec![block], &layout_cfg, &output_cfg)
            .expect("prepare failed");
        assert_eq!(processed.len(), 1);
        let p = &processed[0];
        // 4 payload + 4 CRC
        assert_eq!(p.data.len(), 8);
        assert_eq!(p.address_range.0, 0x1000);
        assert_eq!(p.address_range.1, 0x1000 + 8);
    }

    #[test]
    fn emit_combined_hex_produces_output() {
        let layout_cfg = sample_config();

        let header = Header {
            start_address: 0x2000,
            length: 16,
            crc_location: CrcLocation::Keyword("end".to_string()),
            padding: 0xFF,
        };
        let block = BlockData {
            name: "blk".to_string(),
            header,
            data: vec![0xAA, 0xBB, 0xCC, 0xDD],
        };
        let output_cfg = CombinedOutputConfig {
            record_width: 16,
            format: crate::output::args::OutputFormat::Hex,
            combined_crc: None,
        };
        let processed = prepare_combined_data(vec![block], &layout_cfg, &output_cfg)
            .expect("prepare failed");
        let hex = emit_combined_hex(&processed, &layout_cfg, &output_cfg)
            .expect("emit failed");
        assert!(!hex.is_empty());
        assert!(hex.lines().next().unwrap_or("").starts_with(":"));
    }
}
