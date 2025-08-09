#![allow(dead_code, unused_variables, unused_imports)]

mod error;
mod layout;
mod types;
mod variants;

use crate::error::*;
use clap::Parser;
use std::path::Path;

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
    let filetype = Path::new(filename).extension().and_then(|s| s.to_str());

    let flash_block = match filetype {
        Some("toml") => layout::FlashBlock::<toml::Table>::new(filename, "block")?,
        // Some("yaml") => layout::FlashBlock::<serde_yaml::Mapping>::new(filename, "block")?,
        // Some("json") => layout::FlashBlock::<serde_json::Map<String, serde_json::Value>>::new(filename, "block")?,
        _ => return Err(NvmError::FileError("Unsupported file format".to_string())),
    };

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

    let bytestream = flash_block.build_bytestream(&data_sheet)?;
    println!("Bytestream: {:?}", bytestream);

    Ok(())
}
