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
    let file_content = std::fs::read_to_string(filename)?;
    let filetype = filename.split('.').last().unwrap();

    let mut flash_block = match filetype {
        "toml" => layout::FlashBlock::<toml::Table>::new(&file_content, "block")?,
        // "yaml" => layout::FlashBlock::<serde_yaml::Mapping>::new(&file_content, "block")?,
        // "json" => layout::FlashBlock::<serde_json::Map<String, serde_json::Value>>::new(
        //     &file_content,
        //     "block",
        // )?,
        _ => anyhow::bail!("Unsupported file format"),
    };

    println!("✅ Successfully loaded FlashBlock!");
    println!("Start Address: 0x{:X}", flash_block.start_address());
    println!("Length: 0x{:X}", flash_block.length());
    println!("CRC Polynomial: 0x{:X}", flash_block.crc_poly());
    println!("CRC Location: {:?}", flash_block.crc_location());

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

    // Test the walk_data_section method
    match data_sheet.walk_data_section(&mut flash_block.data_mut()) {
        Ok(_) => {
            println!("✅ Successfully walked data section!");
        }
        Err(e) => {
            println!("❌ Failed to walk data section: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
