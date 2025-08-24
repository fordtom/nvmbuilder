mod error;
mod hex;
mod layout;
mod schema;
mod variants;

use clap::Parser;
use rayon::prelude::*;
use std::path::Path;

use crate::error::*;
use crate::schema::*;
use hex::bytestream_to_hex_string;
use layout::load_layout;
use variants::DataSheet;

fn parse_offset(offset: &str) -> Result<u32, NvmError> {
    let s = offset.trim();
    let (radix, digits) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, hex)
    } else {
        (10, s)
    };

    u32::from_str_radix(&digits.replace("_", ""), radix)
        .map_err(|_| NvmError::MiscError(format!("invalid offset provided: {}", offset)))
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Build flash blocks from layout + Excel data")]
pub struct Args {
    // Positional: at least one block name
    #[arg(value_name = "BLOCK", num_args = 1.., help = "Block name(s) to build")]
    pub blocks: Vec<String>,

    #[arg(
        short = 'l',
        long,
        required = true,
        value_name = "FILE",
        help = "Path to the layout file (TOML/YAML/JSON)"
    )]
    pub layout: String,

    #[arg(
        short = 'x',
        long,
        required = true,
        value_name = "FILE",
        help = "Path to the Excel variants file"
    )]
    pub xlsx: String,

    #[arg(short = 'v', long, value_name = "NAME", help = "Variant column to use")]
    pub variant: Option<String>,

    #[arg(short = 'd', long, help = "Use the Debug column when present")]
    pub debug: bool,

    #[arg(
        short = 'o',
        long,
        value_name = "DIR",
        default_value = "out",
        help = "Output directory for .hex files"
    )]
    pub out: String,

    #[arg(
        long,
        value_name = "OFFSET",
        default_value_t = 0u32,
        value_parser = parse_offset,
        help = "Optional virtual address offset (hex or dec)"
    )]
    pub offset: u32,
}

fn build_block(
    layout: &Config,
    data_sheet: &DataSheet,
    block_name: &str,
    offset: u32,
    out: &str,
) -> Result<(), NvmError> {
    let block = layout
        .blocks
        .get(block_name)
        .ok_or(NvmError::BlockNotFound(block_name.to_string()))?;

    let mut bytestream = block.build_bytestream(&data_sheet, &layout.settings)?;

    let hex_string =
        bytestream_to_hex_string(&mut bytestream, &block.header, &layout.settings, offset)?;

    let out_path = Path::new(out).join(format!("{}.hex", block_name));
    std::fs::write(out_path, hex_string)
        .map_err(|e| NvmError::FileError(format!("failed to write block {}: {}", block_name, e)))?;

    Ok(())
}

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let layout = load_layout(&args.layout)?;
    let data_sheet = DataSheet::new(&args.xlsx, args.variant, args.debug)?;

    std::fs::create_dir_all(&args.out)
        .map_err(|e| NvmError::FileError(format!("failed to create output directory: {}", e)))?;

    args.blocks.par_iter().try_for_each(|block_name| {
        build_block(&layout, &data_sheet, block_name, args.offset, &args.out)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
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
        let offsets: [u32; 2] = [0, 0x1000];

        fs::create_dir_all("out").unwrap();

        for layout_path in layouts {
            let cfg = load_layout(layout_path).expect("failed to parse layout");

            // Try a few option combinations; degrade gracefully if a variant column is missing
            let variant_candidates: [Option<&str>; 2] = [None, Some("VarA")];
            let debug_candidates = [false, true];

            let mut ds_opt: Option<DataSheet> = None;
            for &dbg in &debug_candidates {
                for var in &variant_candidates {
                    let var_opt: Option<String> = var.map(|s| s.to_string());
                    match DataSheet::new("examples/data.xlsx", var_opt, dbg) {
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
            let ds = ds_opt.unwrap_or_else(|| {
                DataSheet::new("examples/data.xlsx", None, false)
                    .expect("Excel open with default columns")
            });

            for &blk in &blocks {
                if !cfg.blocks.contains_key(blk) {
                    continue;
                }
                for &off in &offsets {
                    build_block(&cfg, &ds, blk, off, "out").expect("build_block failed");
                    assert!(Path::new("out").join(format!("{}.hex", blk)).exists());
                }
            }
        }
    }
}
