pub fn format_bytes(bytes: usize) -> String {
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

pub fn format_address_range(start: u32, allocated: u32) -> String {
    let end = start + allocated - 1;
    format!("0x{:08X}-0x{:08X}", start, end)
}

pub fn format_efficiency(used: u32, allocated: u32) -> String {
    if allocated == 0 {
        "0.0%".to_string()
    } else {
        format!("{:.1}%", (used as f64 / allocated as f64) * 100.0)
    }
}
