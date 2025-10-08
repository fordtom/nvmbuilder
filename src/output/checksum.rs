use crate::layout::settings::CrcData;

/// Hand-rolled CRC32 calculation matching the crc crate's NoTable implementation.
/// This removes the need for static state and allows each block to use its own CRC settings.
pub fn calculate_crc(data: &[u8], crc_settings: &CrcData) -> u32 {
    // Initialize CRC based on ref_in
    let mut crc = if crc_settings.ref_in {
        crc_settings.start.reverse_bits()
    } else {
        crc_settings.start
    };

    // Prepare polynomial
    let poly = if crc_settings.ref_in {
        crc_settings.polynomial.reverse_bits()
    } else {
        crc_settings.polynomial
    };

    // Process each byte
    for &byte in data {
        let idx = if crc_settings.ref_in {
            (crc ^ (byte as u32)) & 0xFF
        } else {
            ((crc >> 24) ^ (byte as u32)) & 0xFF
        };

        // Perform 8 rounds of bitwise CRC calculation
        let mut step = idx;
        if crc_settings.ref_in {
            for _ in 0..8 {
                step = (step >> 1) ^ ((step & 1) * poly);
            }
        } else {
            for _ in 0..8 {
                step = (step << 1) ^ (((step >> 31) & 1) * poly);
            }
        }

        crc = if crc_settings.ref_in {
            step ^ (crc >> 8)
        } else {
            step ^ (crc << 8)
        };
    }

    // Finalize
    if crc_settings.ref_in ^ crc_settings.ref_out {
        crc = crc.reverse_bits();
    }

    crc ^ crc_settings.xor_out
}
