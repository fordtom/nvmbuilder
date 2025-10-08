use clap::Parser;

use nvmbuilder::args::Args;
use nvmbuilder::commands;
use nvmbuilder::error::*;
use nvmbuilder::layout;
use nvmbuilder::output;
use nvmbuilder::variant::DataSheet;
use nvmbuilder::visuals;

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let data_sheet = DataSheet::new(&args.variant)?;

    // Warn if variant or debug flags are used without an Excel file
    if data_sheet.is_none() && (args.variant.variant.is_some() || args.variant.debug) {
        eprintln!(
            "Warning: --variant or --debug flag specified without an Excel file (-x). These flags will be ignored."
        );
    }

    // Check if blocks are provided
    let first_block = args
        .layout
        .blocks
        .first()
        .ok_or(layout::errors::LayoutError::NoBlocksProvided)?;

    // This is a temporary fix for the one-time initialisation of the crc
    let first_layout = layout::load_layout(&first_block.file)?;
    output::checksum::init_crc_algorithm(&first_layout.settings.crc);

    std::fs::create_dir_all(&args.output.out).map_err(|e| {
        NvmError::Output(nvmbuilder::output::errors::OutputError::FileError(format!(
            "failed to create output directory: {}",
            e
        )))
    })?;

    let stats = match args.output.combined {
        true => commands::build_single_file(&args, data_sheet.as_ref())?,
        false => commands::build_separate_blocks(&args, data_sheet.as_ref())?,
    };

    if !args.output.quiet {
        if args.output.stats {
            visuals::print_detailed(&stats);
        } else {
            visuals::print_summary(&stats);
        }
    }

    Ok(())
}
