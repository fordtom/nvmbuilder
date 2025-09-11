mod args;
mod error;
mod layout;
mod output;
mod variant;

use clap::Parser;
use rayon::prelude::*;
use std::path::Path;

use args::Args;
use error::*;
use layout::args::BlockNames;
use variant::DataSheet;

fn build_block(input: &BlockNames, data_sheet: &DataSheet, args: &Args) -> Result<(), NvmError> {
    let layout = layout::load_layout(&input.file)?;

    let block = layout
        .blocks
        .get(&input.name)
        .ok_or(NvmError::BlockNotFound(input.name.clone()))?;

    let mut bytestream = block.build_bytestream(data_sheet, &layout.settings)?;

    let hex_string = output::bytestream_to_hex_string(
        &mut bytestream,
        &block.header,
        &layout.settings,
        args.byte_swap,
        args.record_width as usize,
        args.pad_to_end,
    )?;

    let mut name_parts: Vec<String> = Vec::new();
    if !args.prefix.is_empty() {
        name_parts.push(args.prefix.clone());
    }
    name_parts.push(input.name.to_string());
    if !args.suffix.is_empty() {
        name_parts.push(args.suffix.clone());
    }
    let out_filename = format!("{}.hex", name_parts.join("_"));
    let out_path = Path::new(&args.out).join(out_filename);
    std::fs::write(out_path, hex_string)
        .map_err(|e| NvmError::FileError(format!("failed to write block {}: {}", input.name, e)))?;

    Ok(())
}

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let data_sheet = DataSheet::new(&args.variant)?;

    // This is a temporary fix for the one-time initialisation of the crc
    let first_block = args.layout.blocks.first().unwrap();
    let first_layout = layout::load_layout(&first_block.file)?;
    output::checksum::init_crc_algorithm(&first_layout.settings.crc);

    std::fs::create_dir_all(&args.out)
        .map_err(|e| NvmError::FileError(format!("failed to create output directory: {}", e)))?;

    args.layout
        .blocks
        .par_iter()
        .try_for_each(|input| build_block(input, &data_sheet, &args))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // use crate::variant::args::VariantArgs;

    use super::*;
    use std::fs;
    // use std::path::Path;

    #[test]
    fn smoke_build_examples_all_formats_and_options() {
        let layouts = [
            "examples/block.toml",
            "examples/block.yaml",
            "examples/block.json",
        ];
        // let blocks = ["block", "block2", "block3"];
        // let offsets: [u32; 2] = [0, 0x1000];

        fs::create_dir_all("out").unwrap();

        for layout_path in layouts {
            let /*mut*/ cfg = layout::load_layout(layout_path).expect("failed to parse layout");
            output::checksum::init_crc_algorithm(&cfg.settings.crc);

            // Try a few option combinations; degrade gracefully if a variant column is missing
            let variant_candidates: [Option<&str>; 2] = [None, Some("VarA")];
            let debug_candidates = [false, true];

            let mut ds_opt: Option<DataSheet> = None;
            for &dbg in &debug_candidates {
                for var in &variant_candidates {
                    let var_opt: Option<String> = var.map(|s| s.to_string());
                    let var_args = variant::args::VariantArgs {
                        xlsx: "examples/data.xlsx".to_string(),
                        variant: var_opt,
                        debug: dbg,
                        main_sheet: "Main".to_string(),
                    };
                    match DataSheet::new(&var_args) {
                        Ok(ds) => {
                            ds_opt = Some(ds);
                            break;
                        }
                        Err(_) => continue,
                    }
                }
                if ds_opt.is_some() {
                    break;
                }
            }
            // let ds = ds_opt.unwrap_or_else(|| {
            //     let datasheet_args = VariantArgs {
            //         xlsx: "examples/data.xlsx".to_string(),
            //         debug: false,
            //         variant: None,
            //         main_sheet: "Main".to_string(),
            //     };

            //     DataSheet::new(&datasheet_args).expect("Excel open with default columns")
            // });

            // for &blk in &blocks {
            //     if !cfg.blocks.contains_key(blk) {
            //         continue;
            //     }
            //     for &off in &offsets {
            //         cfg.settings.virtual_offset = off;
            //         build_block(
            //             &cfg,
            //             &ds,
            //             blk,
            //             &Args {
            //                 blocks: vec![blk.to_string()],
            //                 layout: layout_path.to_string(),
            //                 xlsx: "examples/data.xlsx".to_string(),
            //                 variant: None,
            //                 debug: false,
            //                 byte_swap: false,
            //                 out: "out".to_string(),
            //                 main_sheet: "Main".to_string(),
            //                 prefix: "PRE".to_string(),
            //                 suffix: "SUF".to_string(),
            //                 record_width: 32,
            //                 pad_to_end: false,
            //             },
            //         )
            //         .expect("build_block failed");
            //         let expected = format!("{}_{}_{}.hex", "PRE", blk, "SUF");
            //         assert!(Path::new("out").join(expected).exists());
            //     }
            // }
        }
    }
}
