use clap::Parser;

#[derive(Parser, Debug)]
pub struct VariantArgs {
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
}
