use nvmbuilder::commands;
use nvmbuilder::variant::DataSheet;

#[path = "common/mod.rs"]
mod common;

#[test]
fn test_build_without_excel() {
    common::ensure_out_dir();

    let layout_path = "examples/block_no_excel.toml";
    common::init_crc_from_layout(layout_path);

    let input = nvmbuilder::layout::args::BlockNames {
        name: "simple_block".to_string(),
        file: layout_path.to_string(),
    };

    // Build args without Excel file
    let args = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![input.clone()],
            strict: false,
        },
        variant: nvmbuilder::variant::args::VariantArgs {
            xlsx: None,
            variant: None,
            debug: false,
            main_sheet: "Main".to_string(),
        },
        output: nvmbuilder::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "TEST".to_string(),
            suffix: "NOEXCEL".to_string(),
            record_width: 32,
            format: nvmbuilder::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    // This should succeed since all values are inline
    commands::generate::build_block_single(&input, None, &args)
        .expect("build should succeed without Excel file");

    common::assert_out_file_exists_custom(
        "simple_block",
        "TEST",
        "NOEXCEL",
        nvmbuilder::output::args::OutputFormat::Hex,
    );
}

#[test]
fn test_error_when_name_without_excel() {
    common::ensure_out_dir();

    // Use a layout that references names from Excel
    let layout_path = "examples/block.toml";
    common::init_crc_from_layout(layout_path);

    let input = nvmbuilder::layout::args::BlockNames {
        name: "block".to_string(),
        file: layout_path.to_string(),
    };

    let args = nvmbuilder::args::Args {
        layout: nvmbuilder::layout::args::LayoutArgs {
            blocks: vec![input.clone()],
            strict: false,
        },
        variant: nvmbuilder::variant::args::VariantArgs {
            xlsx: None,
            variant: None,
            debug: false,
            main_sheet: "Main".to_string(),
        },
        output: nvmbuilder::output::args::OutputArgs {
            out: "out".to_string(),
            prefix: "TEST".to_string(),
            suffix: "ERROR".to_string(),
            record_width: 32,
            format: nvmbuilder::output::args::OutputFormat::Hex,
            combined: false,
            stats: false,
            quiet: true,
        },
    };

    // This should fail with MissingDataSheet error
    let result = commands::generate::build_block_single(&input, None, &args);
    assert!(
        result.is_err(),
        "Expected error when using 'name' without Excel file"
    );

    let err = result.unwrap_err();
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("Missing datasheet")
            || err_str.contains("requires a value from the Excel datasheet"),
        "Error should mention missing datasheet, got: {}",
        err_str
    );
}

#[test]
fn test_warning_validation() {
    // Test that DataSheet::new returns None when no xlsx is provided
    let args_no_excel = nvmbuilder::variant::args::VariantArgs {
        xlsx: None,
        variant: None,
        debug: false,
        main_sheet: "Main".to_string(),
    };

    let result = DataSheet::new(&args_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "DataSheet should be None when no xlsx provided"
    );

    // Test with variant flag (would produce warning in main.rs)
    let args_variant_no_excel = nvmbuilder::variant::args::VariantArgs {
        xlsx: None,
        variant: Some("VarA".to_string()),
        debug: false,
        main_sheet: "Main".to_string(),
    };

    let result = DataSheet::new(&args_variant_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "DataSheet should be None when no xlsx provided, even with variant flag"
    );

    // Test with debug flag (would produce warning in main.rs)
    let args_debug_no_excel = nvmbuilder::variant::args::VariantArgs {
        xlsx: None,
        variant: None,
        debug: true,
        main_sheet: "Main".to_string(),
    };

    let result = DataSheet::new(&args_debug_no_excel).expect("should return Ok(None)");
    assert!(
        result.is_none(),
        "DataSheet should be None when no xlsx provided, even with debug flag"
    );
}
