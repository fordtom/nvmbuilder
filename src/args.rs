use crate::layout::args::LayoutArgs;
use crate::variant::args::VariantArgs;
use clap::Parser;

// Eventually these should be split per section once modules expand and become more complex
#[derive(Parser, Debug)]
#[command(author, version, about = "Build flash blocks from layout + Excel data")]
pub struct Args {
    #[command(flatten)]
    pub layout: LayoutArgs,

    #[command(flatten)]
    pub variant: VariantArgs,

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
