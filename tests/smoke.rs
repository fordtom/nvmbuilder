use nvmbuilder::commands::generate::build_block_single;

#[path = "common/mod.rs"]
mod common;

#[test]
fn smoke_build_examples_all_formats_and_options() {
    common::ensure_out_dir();

    let layouts = [
        "examples/block.toml",
        "examples/block.yaml",
        "examples/block.json",
    ];
    let blocks = ["block", "block2", "block3"];

    for layout_path in layouts {
        common::init_crc_from_layout(layout_path);

        let Some(ds) = common::find_working_datasheet() else {
            continue;
        };

        for &blk in &blocks {
            let cfg = nvmbuilder::layout::load_layout(layout_path).expect("layout loads");
            if !cfg.blocks.contains_key(blk) {
                continue;
            }

            // Hex
            let args_hex = common::build_args(
                layout_path,
                blk,
                nvmbuilder::output::args::OutputFormat::Hex,
            );
            let input = nvmbuilder::layout::args::BlockNames {
                name: blk.to_string(),
                file: layout_path.to_string(),
            };
            build_block_single(&input, &ds, &args_hex).expect("build hex");
            common::assert_out_file_exists(blk, nvmbuilder::output::args::OutputFormat::Hex);

            // Mot
            let args_mot = common::build_args(
                layout_path,
                blk,
                nvmbuilder::output::args::OutputFormat::Mot,
            );
            build_block_single(&input, &ds, &args_mot).expect("build mot");
            common::assert_out_file_exists(blk, nvmbuilder::output::args::OutputFormat::Mot);
        }
    }
}
