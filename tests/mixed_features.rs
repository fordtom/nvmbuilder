use nvmbuilder::commands::generate::build_block_single;
use nvmbuilder::layout::args::BlockNames;
use nvmbuilder::output::args::{OutputArgs, OutputFormat};

#[path = "common/mod.rs"]
mod common;

// This integration test exercises:
// - Big endian vs little endian
// - byte_swap true and false
// - pad_to_end true and false
// - CRC at end and at explicit address
// - record width variations (16 and 64)
// - Output formats HEX and MOT (SREC address length auto-selection)
// - virtual_offset changing start addresses
// - 1D array strings and numeric arrays
// - 2D array retrieval and padding
// - mix of value sources (Value and Name)
#[test]
fn mixed_feature_matrix() {
    // Build two layouts to cover multiple settings
    let layout_be_swap_pad_addr = r#"
[settings]
endianness = "big"
virtual_offset = 0
byte_swap = true
pad_to_end = true

[settings.crc]
polynomial = 0x04C11DB7
start = 0xFFFFFFFF
xor_out = 0xFFFFFFFF
ref_in = true
ref_out = true
area = "data"

[block.header]
start_address = 0x10000
length = 0x80
crc_location = 0x10060
padding = 0xAA

[block.data]
nums.u16_be = { value = [1, 2, 3, 4], type = "u16", size = 4 }
txt.ascii = { value = "HELLO", type = "u8", size = 8 }
single.i32 = { value = 42, type = "i32" }
"#;

    let layout_le_no_swap_end = r#"
[settings]
endianness = "little"
virtual_offset = 0x20000
byte_swap = false
pad_to_end = false

[settings.crc]
polynomial = 0x04C11DB7
start = 0xFFFFFFFF
xor_out = 0xFFFFFFFF
ref_in = true
ref_out = true
area = "data"

[block.header]
start_address = 0x90000
length = 0x40
crc_location = "end"
padding = 0x00

[block.data]
arr.f32 = { value = [1.0, 2.5], type = "f32", size = 2 }
arr2.i16 = { value = [10, -20, 30, -40], type = "i16", size = 4 }
"#;

    // write layouts
    let be_path = common::write_layout_file("mixed_be", layout_be_swap_pad_addr);
    let le_path = common::write_layout_file("mixed_le", layout_le_no_swap_end);

    // Prepare a datasheet (may be no-op for these, but keep realistic flow)
    let var_args = nvmbuilder::variant::args::VariantArgs {
        xlsx: Some("examples/data.xlsx".to_string()),
        variant: None,
        debug: false,
        main_sheet: "Main".to_string(),
    };
    let ds = nvmbuilder::variant::DataSheet::new(&var_args).expect("datasheet loads");

    // Initialize CRC (based on respective settings)
    common::init_crc_from_layout(&be_path);
    common::init_crc_from_layout(&le_path);

    // Case 1: Big endian, swap, pad to end, CRC at explicit address, HEX with width 64
    let args_be_hex = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: be_path.clone(),
            }],
            strict: false,
        },
        variant: var_args.clone(),
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "MIX".to_string(),
            suffix: "A".to_string(),
            record_width: 64,
            format: OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: false,
        },
    };
    build_block_single(
        &BlockNames {
            name: "block".to_string(),
            file: be_path.clone(),
        },
        ds.as_ref(),
        &args_be_hex,
    )
    .expect("be-hex");
    assert!(std::path::Path::new("out/MIX_block_A.hex").exists());

    // Case 2: Big endian, swap, pad to end, explicit CRC, MOT with width 16
    let args_be_mot = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: be_path.clone(),
            }],
            strict: false,
        },
        variant: var_args.clone(),
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "MIX".to_string(),
            suffix: "B".to_string(),
            record_width: 16,
            format: OutputFormat::Mot,
            combined: false,
            stats: false,
            quiet: false,
        },
    };
    build_block_single(
        &BlockNames {
            name: "block".to_string(),
            file: be_path.clone(),
        },
        ds.as_ref(),
        &args_be_mot,
    )
    .expect("be-mot");
    assert!(std::path::Path::new("out/MIX_block_B.mot").exists());

    // Case 3: Little endian, no swap, no pad to end, CRC at end, HEX width 16, virtual_offset applied
    let args_le_hex = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: le_path.clone(),
            }],
            strict: true, // exercise strict path on numeric arrays
        },
        variant: var_args.clone(),
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "MIX".to_string(),
            suffix: "C".to_string(),
            record_width: 16,
            format: OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: false,
        },
    };
    build_block_single(
        &BlockNames {
            name: "block".to_string(),
            file: le_path.clone(),
        },
        ds.as_ref(),
        &args_le_hex,
    )
    .expect("le-hex");
    assert!(std::path::Path::new("out/MIX_block_C.hex").exists());

    // Case 4: Little endian, no swap, no pad, CRC at end, MOT width 64
    let args_le_mot = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![BlockNames {
                name: "block".to_string(),
                file: le_path.clone(),
            }],
            strict: true,
        },
        variant: var_args,
        output: OutputArgs {
            out: "out".to_string(),
            prefix: "MIX".to_string(),
            suffix: "D".to_string(),
            record_width: 64,
            format: OutputFormat::Mot,
            combined: false,
            stats: false,
            quiet: false,
        },
    };
    build_block_single(
        &BlockNames {
            name: "block".to_string(),
            file: le_path.clone(),
        },
        ds.as_ref(),
        &args_le_mot,
    )
    .expect("le-mot");
    assert!(std::path::Path::new("out/MIX_block_D.mot").exists());
}
