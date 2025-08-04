#![allow(dead_code, unused_variables)]

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

fn main() {
    // let args = Args::parse();

    // Test the FlashBlock constructor
    let flash_block = match layout::FlashBlock::new("data/block.toml", "block") {
        Ok(flash_block) => {
            println!("✅ Successfully loaded FlashBlock!");
            println!("Start Address: 0x{:X}", flash_block.start_address());
            println!("Length: 0x{:X}", flash_block.length());
            println!("CRC Polynomial: 0x{:X}", flash_block.crc_poly());
            println!("CRC Location: {:?}", flash_block.crc_location());
            println!("Data entries: {}", flash_block.data().len());
            flash_block
        }
        Err(e) => {
            println!("❌ Failed to load FlashBlock: {:?}", e);
            return;
        }
    };

    // Test the DataSheet constructor
    let data_sheet = match variants::DataSheet::new("data/data.xlsx", Some("VarA"), true) {
        Ok(data_sheet) => {
            println!("✅ Successfully loaded DataSheet!");
            data_sheet
        }
        Err(e) => {
            println!("❌ Failed to load DataSheet: {:?}", e);
            return;
        }
    };

    // Test the walk_data_section method
    match data_sheet.walk_data_section(&mut flash_block.data_mut()) {
        Ok(_) => {
            println!("✅ Successfully walked data section!");
        }
        Err(e) => {
            println!("❌ Failed to walk data section: {:?}", e);
            return;
        }
    }
}
