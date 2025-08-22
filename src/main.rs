#![allow(dead_code, unused_variables, unused_imports)]

mod emit;
mod error;
mod layout;
mod schema;
mod variants;

use crate::error::*;
use clap::Parser;
use std::path::Path;

fn load_config(filename: &str) -> Result<schema::Config, NvmError> {
    let text = std::fs::read_to_string(filename)
        .map_err(|_| NvmError::FileError(format!("failed to open file: {}", filename)))?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let cfg: schema::Config = match ext.as_str() {
        "toml" => toml::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "yaml" | "yml" => serde_yaml::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "json" => serde_json::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
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
    let config: schema::Config = load_config(filename)?;

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

    let bytestream = block.build_bytestream(&data_sheet, &config.settings)?;
    println!("Bytestream: {:?}", bytestream);

    Ok(())
}
