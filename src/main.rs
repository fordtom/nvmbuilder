#![allow(dead_code, unused_variables, unused_imports)]

mod error;
mod layout;
mod types;
mod variants;

use crate::error::*;
use clap::Parser;
use std::fs;
use std::path::Path;

fn load_config(filename: &str) -> Result<types::Config, NvmError> {
    let text = std::fs::read_to_string(filename)
        .map_err(|_| NvmError::FileError(format!("failed to open file: {}", filename)))?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let cfg: types::Config = match ext.as_str() {
        "toml" => toml::from_str(&text)
            .map_err(|_| NvmError::FileError("failed to parse file: ".to_string() + filename))?,
        "yaml" | "yml" => serde_yaml::from_str(&text)
            .map_err(|_| NvmError::FileError("failed to parse file: ".to_string() + filename))?,
        "json" => serde_json::from_str(&text)
            .map_err(|_| NvmError::FileError("failed to parse file: ".to_string() + filename))?,
        _ => return Err(NvmError::FileError("Unsupported file format".to_string())),
    };

    Ok(cfg)
}

#[derive(Parser)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "config.toml",
        help = "Path to the config file (optional)",
        value_name = "FILE"
    )]
    config: String,
}

fn main() -> Result<(), NvmError> {
    // let args = Args::parse();

    let filename = "data/block.toml";
    let block_name = "block";
    let config: types::Config = load_config(filename)?;

    println!("Settings: {:?}", config.settings);

    let block = config
        .blocks
        .get(block_name)
        .ok_or(NvmError::BlockNotFound(block_name.to_string()))?;

    println!("Block header: {:?}", block.header);

    // Test the DataSheet constructor
    let data_sheet = match variants::DataSheet::new("data/data.xlsx", Some("VarA"), true) {
        Ok(data_sheet) => {
            println!("✅ Successfully loaded DataSheet!");
            data_sheet
        }
        Err(e) => {
            println!("❌ Failed to load DataSheet: {:?}", e);
            return Err(e.into());
        }
    };

    // Later: build bytestream from config when ready
    // let bytestream = config.build_bytestream(&data_sheet)?;
    // println!("Bytestream: {:?}", bytestream);

    Ok(())
}
