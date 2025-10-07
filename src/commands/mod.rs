pub mod generate;

use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::output;
use crate::variant::DataSheet;
use crate::writer::write_output;
use rayon::prelude::*;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BlockStat {
    pub name: String,
    pub start_address: u32,
    pub allocated_size: u32,
    pub used_size: u32,
    pub crc_value: u32,
}

#[derive(Debug)]
pub struct BuildStats {
    pub blocks_processed: usize,
    pub total_allocated: usize,
    pub total_used: usize,
    pub total_duration: Duration,
    pub block_stats: Vec<BlockStat>,
}

impl BuildStats {
    pub fn new() -> Self {
        Self {
            blocks_processed: 0,
            total_allocated: 0,
            total_used: 0,
            total_duration: Duration::from_secs(0),
            block_stats: Vec::new(),
        }
    }

    pub fn add_block(&mut self, stat: BlockStat) {
        self.blocks_processed += 1;
        self.total_allocated += stat.allocated_size as usize;
        self.total_used += stat.used_size as usize;
        self.block_stats.push(stat);
    }

    pub fn space_efficiency(&self) -> f64 {
        if self.total_allocated == 0 {
            0.0
        } else {
            (self.total_used as f64 / self.total_allocated as f64) * 100.0
        }
    }
}

pub fn build_separate_blocks(args: &Args, data_sheet: &DataSheet) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    let block_stats: Result<Vec<BlockStat>, NvmError> = args
        .layout
        .blocks
        .par_iter()
        .map(|input| generate::build_block_single(input, data_sheet, args))
        .collect();

    let block_stats = block_stats?;

    let mut stats = BuildStats::new();
    for stat in block_stats {
        stats.add_block(stat);
    }
    stats.total_duration = start_time.elapsed();

    Ok(stats)
}

pub fn build_single_file(args: &Args, data_sheet: &DataSheet) -> Result<BuildStats, NvmError> {
    let start_time = Instant::now();

    let mut ranges = Vec::new();
    let mut block_ranges: Vec<(String, u32, u32)> = Vec::new();
    let mut stats = BuildStats::new();

    for input in &args.layout.blocks {
        let layout = layout::load_layout(&input.file)?;

        let block = layout
            .blocks
            .get(&input.name)
            .ok_or(NvmError::BlockNotFound(input.name.clone()))?;

        let bytestream =
            block.build_bytestream(data_sheet, &layout.settings, args.layout.strict)?;

        let dr = output::bytestream_to_datarange(
            bytestream,
            &block.header,
            &layout.settings,
            layout.settings.byte_swap,
            layout.settings.pad_to_end,
        )?;

        let crc_value = match layout.settings.endianness {
            layout::settings::Endianness::Big => u32::from_be_bytes([
                dr.crc_bytestream[0],
                dr.crc_bytestream[1],
                dr.crc_bytestream[2],
                dr.crc_bytestream[3],
            ]),
            layout::settings::Endianness::Little => u32::from_le_bytes([
                dr.crc_bytestream[0],
                dr.crc_bytestream[1],
                dr.crc_bytestream[2],
                dr.crc_bytestream[3],
            ]),
        };

        stats.add_block(BlockStat {
            name: input.name.clone(),
            start_address: dr.start_address,
            allocated_size: dr.allocated_size,
            used_size: dr.used_size,
            crc_value,
        });

        ranges.push(dr);

        let start = block.header.start_address + layout.settings.virtual_offset;
        let end = start + block.header.length;
        block_ranges.push((input.name.clone(), start, end));
    }

    // Detect overlaps between declared block memory ranges (inclusive start, exclusive end)
    for i in 0..block_ranges.len() {
        for j in (i + 1)..block_ranges.len() {
            let (ref name_a, a_start, a_end) = block_ranges[i];
            let (ref name_b, b_start, b_end) = block_ranges[j];

            let overlap_start = a_start.max(b_start);
            let overlap_end = a_end.min(b_end);

            if overlap_start < overlap_end {
                let overlap_size = overlap_end - overlap_start;
                let msg = format!(
                    "Block '{}' (0x{:08X}-0x{:08X}) overlaps with block '{}' (0x{:08X}-0x{:08X}). Overlap: 0x{:08X}-0x{:08X} ({} bytes)",
                    name_a,
                    a_start,
                    a_end - 1,
                    name_b,
                    b_start,
                    b_end - 1,
                    overlap_start,
                    overlap_end - 1,
                    overlap_size
                );
                return Err(NvmError::BlockOverlapError(msg));
            }
        }
    }

    let hex_string = output::emit_hex(
        &ranges,
        args.output.record_width as usize,
        args.output.format,
    )?;

    write_output(&args.output, "combined", &hex_string)?;

    stats.total_duration = start_time.elapsed();

    Ok(stats)
}
