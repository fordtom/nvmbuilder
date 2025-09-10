use crate::error::*;
use clap::Parser;

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
        long = "offset",
        value_name = "OFFSET",
        hide = true,
        help = "REMOVED: Use file-level 'offset = <VALUE>' in layout file"
    )]
    pub legacy_offset: Option<String>,

    #[arg(
        long,
        value_name = "STR",
        default_value = "",
        help = "Optional prefix to prepend to each block name in output filename"
    )]
    pub prefix: String,

    #[arg(
        long,
        value_name = "STR",
        default_value = "",
        help = "Optional suffix to append to each block name in output filename"
    )]
    pub suffix: String,

    #[arg(
        long,
        value_name = "N",
        default_value_t = 32u16,
        value_parser = clap::value_parser!(u16).range(1..=64),
        help = "Number of bytes per HEX data record (1..=64)"
    )]
    pub record_width: u16,

    #[arg(long, help = "Pad output HEX to the full block length")]
    pub pad_to_end: bool,
}

impl Args {
    pub fn validate_legacy_flags(&self) -> Result<(), NvmError> {
        if let Some(val) = &self.legacy_offset {
            return Err(NvmError::MiscError(format!(
                "--offset flag removed. Add 'offset = {}' to your layout file instead.",
                val
            )));
        }
        Ok(())
    }
}
