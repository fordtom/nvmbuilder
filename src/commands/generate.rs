use crate::args::Args;
use crate::error::NvmError;
use crate::layout;
use crate::layout::args::BlockNames;
use crate::variant::DataSheet;
use crate::writer::write_output;

pub fn build_block_single(
    input: &BlockNames,
    data_sheet: &DataSheet,
    args: &Args,
) -> Result<(), NvmError> {
    let layout = layout::load_layout(&input.file)?;

    let block = layout
        .blocks
        .get(&input.name)
        .ok_or(NvmError::BlockNotFound(input.name.clone()))?;

    let mut bytestream =
        block.build_bytestream(data_sheet, &layout.settings, args.layout.strict)?;

    let hex_string = crate::output::bytestream_to_hex_string(
        &mut bytestream,
        &block.header,
        &layout.settings,
        layout.settings.byte_swap,
        args.output.record_width as usize,
        layout.settings.pad_to_end,
        args.output.format,
    )?;

    write_output(&args.output, &input.name, &hex_string)?;

    Ok(())
}
