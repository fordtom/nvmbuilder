use crate::error::*;
use clap::{Parser, builder::ValueParser};

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

// Eventually these should be split per section once modules expand and become more complex
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

    #[arg(
        long,
        value_name = "NAME",
        default_value = "Main",
        help = "Main sheet name in Excel"
    )]
    pub main_sheet: String,

    #[arg(short = 'v', long, value_name = "NAME", help = "Variant column to use")]
    pub variant: Option<String>,

    #[arg(short = 'd', long, help = "Use the Debug column when present")]
    pub debug: bool,

    #[arg(long, help = "Swap bytes in place (for TI)")]
    pub byte_swap: bool,

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

    #[arg(
        long,
        value_name = "N",
        default_value_t = 32u16,
        value_parser = clap::value_parser!(u16).range(1..=64),
        help = "Number of bytes per HEX data record (1..=64)"
    )]
    pub record_width: u16,
}
