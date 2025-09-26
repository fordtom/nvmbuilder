pub mod generate;

use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::output;
use crate::writer::write_output;
use crate::variant::DataSheet;
use rayon::prelude::*;

pub fn build_separate_blocks(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    args.layout
        .blocks
        .par_iter()
        .try_for_each(|input| generate::build_block_single(input, data_sheet, args))
}

pub fn build_single_file(args: &Args, data_sheet: &DataSheet) -> Result<(), NvmError> {
    let mut payloads: Vec<Vec<u8>> = Vec::new();
    let mut ranges = Vec::new();

    for input in &args.layout.blocks {
        let layout = layout::load_layout(&input.file)?;

        let block = layout
            .blocks
            .get(&input.name)
            .ok_or(NvmError::BlockNotFound(input.name.clone()))?;

        let mut bytestream =
            block.build_bytestream(data_sheet, &layout.settings, args.layout.strict)?;

        payloads.push(bytestream);
        let bs_ref = payloads.last_mut().expect("payloads not empty");
        let dr = output::bytestream_to_hex_string(
            bs_ref,
            &block.header,
            &layout.settings,
            layout.settings.byte_swap,
            layout.settings.pad_to_end,
        )?;
        ranges.push(dr);
    }

    let hex_string = output::emit_hex(&ranges, args.output.record_width as usize, args.output.format)?;

    write_output(&args.output, "combined", &hex_string)
}
