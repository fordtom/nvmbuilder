#![allow(dead_code, unused_variables, unused_imports)]

mod layout;
mod variants;

use clap::Parser;

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

fn main() -> anyhow::Result<()> {
    // let args = Args::parse();

    let filename = "data/block.toml";
    let filetype = filename.split('.').last().unwrap();

    let flash_block = match filetype {
        "toml" => layout::FlashBlock::<toml::Table>::new(filename, "block")?,
        // "yaml" => layout::FlashBlock::<serde_yaml::Mapping>::new(filename, "block")?,
        // "json" => layout::FlashBlock::<serde_json::Map<String, serde_json::Value>>::new(filename, "block")?,
        _ => anyhow::bail!("Unsupported file format"),
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

    flash_block.build_bytestream(&data_sheet)?;

    Ok(())
}
