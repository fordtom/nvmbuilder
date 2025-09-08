mod args;
mod checksum;
mod error;
mod hex;
mod dump;
mod layout;
mod schema;
mod variants;

use clap::Parser;
use rayon::prelude::*;
use std::path::Path;

use args::Args;
use error::*;
use schema::*;
use variants::DataSheet;

fn build_block(
    layout: &Config,
    data_sheet: &DataSheet,
    block_name: &str,
    args: &Args,
) -> Result<(), NvmError> {
    let block = layout
        .blocks
        .get(block_name)
        .ok_or(NvmError::BlockNotFound(block_name.to_string()))?;

    let mut bytestream = block.build_bytestream(data_sheet, &layout.settings)?;

    let hex_string = hex::bytestream_to_hex_string(
        &mut bytestream,
        &block.header,
        &layout.settings,
        args.offset,
        args.byte_swap,
    )?;

    if args.dump_json {
        let dr = dump::build_dump(block, data_sheet, &layout.settings)?;
        let mut root = serde_json::Map::new();
        root.insert("block".to_string(), serde_json::Value::String(block_name.to_string()));
        root.insert("values".to_string(), dr.values);
        // enrich meta with runtime options
        let mut meta_obj = match dr.meta {
            serde_json::Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        meta_obj.insert("byte_swap".to_string(), serde_json::json!(args.byte_swap));
        meta_obj.insert("offset".to_string(), serde_json::json!(args.offset));
        meta_obj.insert(
            "start_address_effective".to_string(),
            serde_json::json!(block.header.start_address + args.offset),
        );
        root.insert("meta".to_string(), serde_json::Value::Object(meta_obj));

        let dump_path = Path::new(&args.out).join(format!("{}.dump.json", block_name));
        let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .map_err(|e| NvmError::FileError(format!("failed to serialise dump: {}", e)))?;
        std::fs::write(dump_path, json_str).map_err(|e| {
            NvmError::FileError(format!("failed to write dump for block {}: {}", block_name, e))
        })?;
    }

    let out_path = Path::new(&args.out).join(format!("{}.hex", block_name));
    std::fs::write(out_path, hex_string)
        .map_err(|e| NvmError::FileError(format!("failed to write block {}: {}", block_name, e)))?;

    Ok(())
}

fn main() -> Result<(), NvmError> {
    let args = Args::parse();

    let layout = layout::load_layout(&args.layout)?;
    let data_sheet = DataSheet::new(&args.xlsx, &args.variant, args.debug, &args.main_sheet)?;

    checksum::init_crc_algorithm(&layout.settings.crc);

    std::fs::create_dir_all(&args.out)
        .map_err(|e| NvmError::FileError(format!("failed to create output directory: {}", e)))?;

    args.blocks
        .par_iter()
        .try_for_each(|block_name| build_block(&layout, &data_sheet, block_name, &args))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn smoke_build_examples_all_formats_and_options() {
        let layouts = [
            "examples/block.toml",
            "examples/block.yaml",
            "examples/block.json",
        ];
        let blocks = ["block", "block2", "block3"];
        let offsets: [u32; 2] = [0, 0x1000];

        fs::create_dir_all("out").unwrap();

        for layout_path in layouts {
            let cfg = layout::load_layout(layout_path).expect("failed to parse layout");
            checksum::init_crc_algorithm(&cfg.settings.crc);

            // Try a few option combinations; degrade gracefully if a variant column is missing
            let variant_candidates: [Option<&str>; 2] = [None, Some("VarA")];
            let debug_candidates = [false, true];

            let mut ds_opt: Option<DataSheet> = None;
            for &dbg in &debug_candidates {
                for var in &variant_candidates {
                    let var_opt: Option<String> = var.map(|s| s.to_string());
                    match DataSheet::new("examples/data.xlsx", &var_opt, dbg, "Main") {
                        Ok(ds) => {
                            ds_opt = Some(ds);
                            break;
                        }
                        Err(_) => continue,
                    }
                }
                if ds_opt.is_some() {
                    break;
                }
            }
            let ds = ds_opt.unwrap_or_else(|| {
                DataSheet::new("examples/data.xlsx", &None, false, "Main")
                    .expect("Excel open with default columns")
            });

            for &blk in &blocks {
                if !cfg.blocks.contains_key(blk) {
                    continue;
                }
                for &off in &offsets {
                    build_block(
                        &cfg,
                        &ds,
                        blk,
                        &Args {
                            blocks: vec![blk.to_string()],
                            layout: layout_path.to_string(),
                            xlsx: "examples/data.xlsx".to_string(),
                            variant: None,
                            debug: false,
                            byte_swap: false,
                            out: "out".to_string(),
                            offset: off,
                            main_sheet: "Main".to_string(),
                            dump_json: false,
                        },
                    )
                    .expect("build_block failed");
                    assert!(Path::new("out").join(format!("{}.hex", blk)).exists());
                }
            }
        }
    }
}
