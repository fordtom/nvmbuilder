use crate::commands::BuildStats;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};

pub fn print_summary(stats: &BuildStats) {
    println!(
        "✓ Built {} blocks in {}ms ({:.1}% efficiency)",
        stats.blocks_processed,
        stats.total_duration.as_millis(),
        stats.space_efficiency()
    );
}

pub fn print_detailed(stats: &BuildStats) {
    let mut summary_table = Table::new();
    summary_table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Build Summary")
                .add_attribute(Attribute::Bold)
                .set_alignment(comfy_table::CellAlignment::Left),
            Cell::new(""),
        ]);

    summary_table.add_row(vec![
        "Build Time",
        &format!("{}ms", stats.total_duration.as_millis()),
    ]);
    summary_table.add_row(vec![
        "Blocks Processed",
        &format!("{}", stats.blocks_processed),
    ]);
    summary_table.add_row(vec![
        "Total Allocated",
        &format_bytes(stats.total_allocated),
    ]);
    summary_table.add_row(vec!["Total Used", &format_bytes(stats.total_used)]);
    summary_table.add_row(vec![
        "Space Efficiency",
        &format!("{:.1}%", stats.space_efficiency()),
    ]);

    println!("{summary_table}\n");

    let mut detail_table = Table::new();
    detail_table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Block").add_attribute(Attribute::Bold),
            Cell::new("Address Range").add_attribute(Attribute::Bold),
            Cell::new("Used/Alloc").add_attribute(Attribute::Bold),
            Cell::new("Efficiency").add_attribute(Attribute::Bold),
            Cell::new("CRC Value").add_attribute(Attribute::Bold),
        ]);

    for block in &stats.block_stats {
        detail_table.add_row(vec![
            Cell::new(&block.name),
            Cell::new(format_address_range(
                block.start_address,
                block.allocated_size,
            )),
            Cell::new(format!(
                "{}/{}",
                format_bytes(block.used_size as usize),
                format_bytes(block.allocated_size as usize)
            )),
            Cell::new(format_efficiency(block.used_size, block.allocated_size)),
            Cell::new(format!("0x{:08X}", block.crc_value)),
        ]);
    }

    println!("{detail_table}");
}

fn format_bytes(bytes: usize) -> String {
    let s = bytes.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect::<String>() + " bytes"
}

fn format_address_range(start: u32, allocated: u32) -> String {
    let end = start + allocated - 1;
    format!("0x{:08X}-0x{:08X}", start, end)
}

fn format_efficiency(used: u32, allocated: u32) -> String {
    if allocated == 0 {
        "0.0%".to_string()
    } else {
        format!("{:.1}%", (used as f64 / allocated as f64) * 100.0)
    }
}
