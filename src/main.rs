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
    use std::time::Instant;

    fn rel<P: AsRef<Path>>(p: P) -> String {
        p.as_ref().to_string_lossy().into_owned()
    }

    #[test]
    fn builds_block_from_toml_examples_and_writes_hex() {
        let layout_path = rel("examples/block.toml");
        let xlsx_path = rel("examples/data.xlsx");
        let layout = load_layout(&layout_path).expect("failed to parse TOML layout");
        let data_sheet = DataSheet::new(&xlsx_path, None, false).expect("failed to open Excel");

        fs::create_dir_all("out").unwrap();
        build_block(&layout, &data_sheet, "block", 0, "out").expect("build_block failed");

        let hex_path = Path::new("out").join("block.hex");
        assert!(hex_path.exists(), "hex file not created");
        let content = fs::read_to_string(&hex_path).expect("failed to read hex file");

        assert!(content.len() > 100, "hex output unexpectedly small");
        assert!(
            content.contains("48656C6C6F2C20776F726C6421C0C0C0"),
            "expected string bytes not found in HEX"
        );
    }

    #[test]
    fn cross_format_hex_equality_for_block() {
        let xlsx_path = rel("examples/data.xlsx");
        let ds = DataSheet::new(&xlsx_path, None, false).expect("failed to open Excel");

        let layout_toml = load_layout(&rel("examples/block.toml")).expect("toml parse");
        let layout_yaml = load_layout(&rel("examples/block.yaml")).expect("yaml parse");
        let layout_json = load_layout(&rel("examples/block.json")).expect("json parse");

        let compute_hex = |cfg: &Config| -> String {
            let block = cfg.blocks.get("block").expect("block present");
            let mut bs = block
                .build_bytestream(&ds, &cfg.settings)
                .expect("bytestream");
            bytestream_to_hex_string(&mut bs, &block.header, &cfg.settings, 0)
                .expect("hex generation")
        };

        let ht = compute_hex(&layout_toml);
        let hy = compute_hex(&layout_yaml);
        let hj = compute_hex(&layout_json);

        assert_eq!(ht, hy, "TOML vs YAML hex differ");
        assert_eq!(ht, hj, "TOML vs JSON hex differ");
    }

    #[test]
    fn builds_all_blocks_from_toml_examples() {
        let layout_path = rel("examples/block.toml");
        let xlsx_path = rel("examples/data.xlsx");
        let layout = load_layout(&layout_path).expect("failed to parse TOML layout");
        let data_sheet = DataSheet::new(&xlsx_path, None, false).expect("failed to open Excel");

        fs::create_dir_all("out").unwrap();
        for name in ["block", "block2", "block3"] {
            build_block(&layout, &data_sheet, name, 0, "out").expect("build_block failed");
            assert!(
                Path::new("out").join(format!("{}.hex", name)).exists(),
                "missing hex for {}",
                name
            );
        }
    }

    #[test]
    fn perf_smoke_build_block_multiple_times() {
        let threshold_ms = std::env::var("NVM_TEST_PERF_MS")
            .ok()
            .and_then(|v| v.parse::<u128>().ok());
        if threshold_ms.is_none() {
            return;
        }
        let threshold_ms = threshold_ms.unwrap();

        let layout = load_layout(&rel("examples/block.toml")).expect("layout parse");
        let ds = DataSheet::new(&rel("examples/data.xlsx"), None, false).expect("open Excel");

        let iters = 3u32;
        let mut total_ms: u128 = 0;
        for _ in 0..iters {
            let mut bs = {
                let block = layout.blocks.get("block").unwrap();
                block
                    .build_bytestream(&ds, &layout.settings)
                    .expect("bytestream")
            };
            let start = Instant::now();
            let _hex = {
                let block = layout.blocks.get("block").unwrap();
                bytestream_to_hex_string(&mut bs, &block.header, &layout.settings, 0)
                    .expect("hex generation")
            };
            total_ms += start.elapsed().as_millis();
        }
        let avg_ms = total_ms / iters as u128;
        assert!(
            avg_ms <= threshold_ms,
            "avg encode time {}ms > threshold {}ms",
            avg_ms,
            threshold_ms
        );
    }
}
