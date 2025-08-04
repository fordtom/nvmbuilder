use clap::Parser;

mod layout;
mod variants;

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
    match layout::FlashBlock::new("data/block.toml", "block") {
        Ok(flash_block) => {
            println!("✅ Successfully loaded FlashBlock!");
            println!("Start Address: 0x{:X}", flash_block.start_address());
            println!("Length: 0x{:X}", flash_block.length());
            println!("CRC Polynomial: 0x{:X}", flash_block.crc_poly());
            println!("CRC Location: {:?}", flash_block.crc_location());
            println!("Data entries: {}", flash_block.data().len());
        }
        Err(e) => {
            println!("❌ Failed to load FlashBlock: {:?}", e);
        }
    }
}
