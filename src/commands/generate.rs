use crate::args::Args;
use crate::commands::stats::BlockStat;
use crate::error::NvmError;
use crate::layout;
use crate::layout::args::BlockNames;
use crate::layout::settings::Endianness;
use crate::variant::DataSheet;
use crate::writer::write_output;

pub fn build_block_single(
    input: &BlockNames,
    data_sheet: &DataSheet,
    args: &Args,
) -> Result<BlockStat, NvmError> {
    let layout = layout::load_layout(&input.file)?;

    let block = layout
        .blocks
        .get(&input.name)
        .ok_or(NvmError::BlockNotFound(input.name.clone()))?;

    let bytestream = block.build_bytestream(data_sheet, &layout.settings, args.layout.strict)?;

    let data_range = crate::output::bytestream_to_datarange(
        bytestream,
        &block.header,
        &layout.settings,
        layout.settings.byte_swap,
        layout.settings.pad_to_end,
    )?;

    let hex_string = crate::output::emit_hex(
        &[data_range.clone()],
        args.output.record_width as usize,
        args.output.format,
    )?;

    write_output(&args.output, &input.name, &hex_string)?;

    let crc_value = match layout.settings.endianness {
        Endianness::Big => u32::from_be_bytes([
            data_range.crc_bytestream[0],
            data_range.crc_bytestream[1],
            data_range.crc_bytestream[2],
            data_range.crc_bytestream[3],
        ]),
        Endianness::Little => u32::from_le_bytes([
            data_range.crc_bytestream[0],
            data_range.crc_bytestream[1],
            data_range.crc_bytestream[2],
            data_range.crc_bytestream[3],
        ]),
    };

    Ok(BlockStat {
        name: input.name.clone(),
        start_address: data_range.start_address,
        allocated_size: data_range.allocated_size,
        used_size: data_range.used_size,
        crc_value,
    })
}
