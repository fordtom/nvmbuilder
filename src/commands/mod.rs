pub mod generate;

use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::output;
use crate::variant::DataSheet;
use crate::writer::write_output;
use rayon::prelude::*;

pub fn build_separate_blocks(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    args.layout
        .blocks
        .par_iter()
        .try_for_each(|input| generate::build_block_single(input, data_sheet, args))
}

pub fn build_single_file(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    let mut ranges = Vec::new();
    let mut block_ranges: Vec<(String, u32, u32)> = Vec::new();

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

    write_output(&args.output, "combined", &hex_string)
}
