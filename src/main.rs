use clap::Parser;

use nvmbuilder::args::Args;
use nvmbuilder::commands;
use nvmbuilder::error::*;
use nvmbuilder::layout;
use nvmbuilder::output;
use nvmbuilder::variant::DataSheet;

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let data_sheet = DataSheet::new(&args.variant)?;

    // This is a temporary fix for the one-time initialisation of the crc
    let first_block = args.layout.blocks.first().unwrap();
    let first_layout = layout::load_layout(&first_block.file)?;
    output::checksum::init_crc_algorithm(&first_layout.settings.crc);

    std::fs::create_dir_all(&args.output.out)
        .map_err(|e| NvmError::FileError(format!("failed to create output directory: {}", e)))?;

    commands::build_separate_blocks(&args, &data_sheet)?;

    Ok(())
}
