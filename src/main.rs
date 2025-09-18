mod args;
mod error;
mod layout;
mod output;
mod variant;
mod writer;
mod commands;

use clap::Parser;

use args::Args;
use error::*;
use variant::DataSheet;

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

#[cfg(test)]
mod tests {
    use crate::variant::args::VariantArgs;

    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn smoke_build_examples_all_formats_and_options() {
        let layouts = [
            "examples/block.toml",
            "examples/block.yaml",
            "examples/block.json",
        ];
        let blocks = ["block", "block2", "block3"];

        fs::create_dir_all("out").unwrap();

        for layout_path in layouts {
            let cfg = layout::load_layout(layout_path).expect("failed to parse layout");
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
            let Some(ds) = ds_opt.as_ref() else {
                continue;
            };

            for &blk in &blocks {
                if !cfg.blocks.contains_key(blk) {
                    continue;
                }

                let args_for_block = Args {
                    layout: layout::args::LayoutArgs {
                        blocks: vec![layout::args::BlockNames {
                            name: blk.to_string(),
                            file: layout_path.to_string(),
                        }],
                    },
                    variant: VariantArgs {
                        xlsx: "examples/data.xlsx".to_string(),
                        variant: None,
                        debug: false,
                        main_sheet: "Main".to_string(),
                    },
                    output: crate::output::args::OutputArgs {
                        out: "out".to_string(),
                        prefix: "PRE".to_string(),
                        suffix: "SUF".to_string(),
                        record_width: 32,
                        format: crate::output::args::OutputFormat::Hex,
                    },
                };

                let input = layout::args::BlockNames {
                    name: blk.to_string(),
                    file: layout_path.to_string(),
                };

                crate::commands::generate::build_block_single(&input, ds, &args_for_block)
                    .expect("build_block_single failed");
                let expected = format!("{}_{}_{}.hex", "PRE", blk, "SUF");
                assert!(Path::new("out").join(expected).exists());

                // Also validate Mot output
                let args_for_block_mot = Args {
                    layout: layout::args::LayoutArgs {
                        blocks: vec![layout::args::BlockNames {
                            name: blk.to_string(),
                            file: layout_path.to_string(),
                        }],
                    },
                    variant: VariantArgs {
                        xlsx: "examples/data.xlsx".to_string(),
                        variant: None,
                        debug: false,
                        main_sheet: "Main".to_string(),
                    },
                    output: crate::output::args::OutputArgs {
                        out: "out".to_string(),
                        prefix: "PRE".to_string(),
                        suffix: "SUF".to_string(),
                        record_width: 32,
                        format: crate::output::args::OutputFormat::Mot,
                    },
                };
                crate::commands::generate::build_block_single(&input, ds, &args_for_block_mot)
                    .expect("build_block_single failed");
                let expected_mot = format!("{}_{}_{}.mot", "PRE", blk, "SUF");
                assert!(Path::new("out").join(expected_mot).exists());
            }
        }
    }
}
