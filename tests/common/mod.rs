#![allow(dead_code)]

use std::fs;
use std::path::Path;

use nvmbuilder::args::Args;
use nvmbuilder::layout::args::{BlockNames, LayoutArgs};
use nvmbuilder::output::args::{OutputArgs, OutputFormat};
use nvmbuilder::variant::{self, DataSheet};

pub fn ensure_out_dir() {
    fs::create_dir_all("out").unwrap();
}

pub fn write_layout_file(file_stem: &str, contents: &str) -> String {
    ensure_out_dir();
    let path = format!("out/{}.toml", file_stem);
    std::fs::write(&path, contents).expect("write layout file");
    path
}

pub fn build_args(layout_path: &str, block_name: &str, format: OutputFormat) -> Args {
    Args {
        layout: LayoutArgs {
            blocks: vec![BlockNames {
                name: block_name.to_string(),
                file: layout_path.to_string(),
            }],
            strict: false,
        },
        variant: variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: None,
            debug: false,
            main_sheet: "Main".to_string(),
        },
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "PRE".to_string(),
            suffix: "SUF".to_string(),
            record_width: 32,
            format,
            combined: false,
            stats: false,
            quiet: false,
        },
    }
}

pub fn find_working_datasheet() -> Option<DataSheet> {
    let variant_candidates: [Option<&str>; 2] = [None, Some("VarA")];
    let debug_candidates = [false, true];

    for &dbg in &debug_candidates {
        for var in &variant_candidates {
            let var_opt: Option<String> = var.map(|s| s.to_string());
            let var_args = variant::args::VariantArgs {
                xlsx: Some("examples/data.xlsx".to_string()),
                variant: var_opt,
                debug: dbg,
                main_sheet: "Main".to_string(),
            };
            if let Ok(Some(ds)) = DataSheet::new(&var_args) {
                return Some(ds);
            }
        }
    }
    None
}

pub fn assert_out_file_exists(block_name: &str, format: OutputFormat) {
    let ext = match format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let expected = format!("{}_{}_{}.{}", "PRE", block_name, "SUF", ext);
    assert!(Path::new("out").join(expected).exists());
}

pub fn assert_out_file_exists_custom(
    block_name: &str,
    prefix: &str,
    suffix: &str,
    format: OutputFormat,
) {
    let ext = match format {
        OutputFormat::Hex => "hex",
        OutputFormat::Mot => "mot",
    };
    let expected = format!("{}_{}_{}.{}", prefix, block_name, suffix, ext);
    assert!(Path::new("out").join(expected).exists());
}

pub fn build_args_for_layouts(layouts: Vec<BlockNames>, format: OutputFormat) -> Args {
    Args {
        layout: LayoutArgs {
            blocks: layouts,
            strict: false,
        },
        variant: variant::args::VariantArgs {
            xlsx: Some("examples/data.xlsx".to_string()),
            variant: None,
            debug: false,
            main_sheet: "Main".to_string(),
        },
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "PRE".to_string(),
            suffix: "SUF".to_string(),
            record_width: 32,
            format,
            combined: true,
            stats: false,
            quiet: false,
        },
    }
}
