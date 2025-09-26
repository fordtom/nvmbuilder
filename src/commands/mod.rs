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

		// Track address ranges for overlap detection (half-open interval [start, end))
		let start = block.header.start_address + layout.settings.virtual_offset;
		let end = start
			.checked_add(block.header.length)
			.ok_or_else(|| NvmError::MiscError("Block end address overflowed u32".to_string()))?;
		block_ranges.push((input.name.clone(), start, end));
    }

	// Detect overlapping blocks in combined output
	for i in 0..block_ranges.len() {
		let (ref a_name, a_start, a_end) = block_ranges[i];
		for j in (i + 1)..block_ranges.len() {
			let (ref b_name, b_start, b_end) = block_ranges[j];
			let overlap_start = a_start.max(b_start);
			let overlap_end = a_end.min(b_end);
			if overlap_start < overlap_end {
				let overlap_size = overlap_end - overlap_start;
				return Err(NvmError::BlockOverlapError(format!(
					"Block '{}' (0x{:X}-0x{:X}) overlaps with block '{}' (0x{:X}-0x{:X}); overlap 0x{:X}-0x{:X} ({} bytes)",
					a_name,
					a_start,
					a_end.saturating_sub(1),
					b_name,
					b_start,
					b_end.saturating_sub(1),
					overlap_start,
					overlap_end.saturating_sub(1),
					overlap_size
				)));
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
