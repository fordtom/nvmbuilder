use std::io::Write;

#[path = "common/mod.rs"]
mod common;

#[test]
fn strict_conversions_success() {
    common::ensure_out_dir();

    let layout_toml = r#"
[settings]
endianness = "little"
virtual_offset = 0
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
start_address = 0x80000
length = 0x100
crc_location = "end"
padding = 0x00

[block.data]
ok.float_exact_to_i16 = { value = 42.0, type = "i16" }
ok.int_exact_to_f32   = { value = 16777216, type = "f32" }
"#;

    let path = std::path::Path::new("out").join("test_strict_ok.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(layout_toml.as_bytes()).unwrap();

    let cfg = nvmbuilder::layout::load_layout(path.to_str().unwrap()).expect("parse ok layout");
    let block = cfg.blocks.get("block").expect("block present");

    let var_args = nvmbuilder::variant::args::VariantArgs {
        xlsx: "examples/data.xlsx".to_string(),
        variant: None,
        debug: false,
        main_sheet: "Main".to_string(),
    };
    let ds = nvmbuilder::variant::DataSheet::new(&var_args).expect("datasheet loads");

    let bytes = block
        .build_bytestream(&ds, &cfg.settings, true)
        .expect("strict conversions should succeed");
    assert!(!bytes.is_empty());
}

#[test]
fn strict_conversions_fail_fractional_float_to_int() {
    common::ensure_out_dir();

    let layout_toml = r#"
[settings]
endianness = "little"
virtual_offset = 0
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
start_address = 0x80000
length = 0x100
crc_location = "end"
padding = 0x00

[block.data]
bad.frac_to_u8 = { value = 1.5, type = "u8" }
"#;

    let path = std::path::Path::new("out").join("test_strict_bad_frac.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(layout_toml.as_bytes()).unwrap();

    let cfg = nvmbuilder::layout::load_layout(path.to_str().unwrap()).expect("parse bad layout");
    let block = cfg.blocks.get("block").expect("block present");

    let var_args = nvmbuilder::variant::args::VariantArgs {
        xlsx: "examples/data.xlsx".to_string(),
        variant: None,
        debug: false,
        main_sheet: "Main".to_string(),
    };
    let ds = nvmbuilder::variant::DataSheet::new(&var_args).expect("datasheet loads");

    let res = block.build_bytestream(&ds, &cfg.settings, true);
    assert!(
        res.is_err(),
        "strict mode should reject fractional float to int"
    );
}

#[test]
fn strict_conversions_fail_large_int_to_f64_lossy() {
    common::ensure_out_dir();

    let layout_toml = r#"
[settings]
endianness = "little"
virtual_offset = 0
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
start_address = 0x80000
length = 0x100
crc_location = "end"
padding = 0x00

[block.data]
bad.large_int_to_f64 = { value = 9007199254740993, type = "f64" }
"#;

    let path = std::path::Path::new("out").join("test_strict_bad_large.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(layout_toml.as_bytes()).unwrap();

    let cfg = nvmbuilder::layout::load_layout(path.to_str().unwrap()).expect("parse bad layout");
    let block = cfg.blocks.get("block").expect("block present");

    let var_args = nvmbuilder::variant::args::VariantArgs {
        xlsx: "examples/data.xlsx".to_string(),
        variant: None,
        debug: false,
        main_sheet: "Main".to_string(),
    };
    let ds = nvmbuilder::variant::DataSheet::new(&var_args).expect("datasheet loads");

    let res = block.build_bytestream(&ds, &cfg.settings, true);
    assert!(
        res.is_err(),
        "strict mode should reject lossy int to f64 conversion"
    );
}
