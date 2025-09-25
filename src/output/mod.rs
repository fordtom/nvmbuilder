pub mod args;
pub mod checksum;

use crate::error::*;
use crate::layout::header::{CrcLocation, Header};
use crate::layout::settings::{CrcArea, Endianness, Settings};
use crate::output::args::OutputFormat;

use bin_file::{BinFile, IHexFormat};

#[derive(Debug, Clone)]
pub struct DataRange<'a> {
    pub start_address: u32,
    pub bytestream: &'a [u8],
    pub crc_address: u32,
    pub crc_bytestream: &'a [u8],
}

fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
}

fn validate_crc_location(length: usize, header: &Header) -> Result<u32, NvmError> {
    let crc_offset = match &header.crc_location {
        CrcLocation::Address(address) => {
            let crc_offset = address.checked_sub(header.start_address).ok_or_else(|| {
                NvmError::HexOutputError("CRC address before block start.".to_string())
            })?;

            if crc_offset < length as u32 {
                return Err(NvmError::HexOutputError(
                    "CRC overlaps with payload.".to_string(),
                ));
            }

            crc_offset
        }
        CrcLocation::Keyword(option) => match option.as_str() {
            "end" => (length as u32 + 3) & !3,
            _ => {
                return Err(NvmError::HexOutputError(format!(
                    "Invalid CRC location: {}",
                    option
                )));
            }
        },
    };

    if header.length < crc_offset + 4 {
        return Err(NvmError::HexOutputError(
            "CRC location would overrun block.".to_string(),
        ));
    }

    Ok(crc_offset)
}

pub fn bytestream_to_hex_string(
    bytestream: &mut Vec<u8>,
    header: &Header,
    settings: &Settings,
    byte_swap: bool,
    record_width: usize,
    pad_to_end: bool,
    format: OutputFormat,
) -> Result<String, NvmError> {
    if bytestream.len() > header.length as usize {
        return Err(NvmError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    // Apply optional byte swap across the entire stream before CRC
    if byte_swap {
        byte_swap_inplace(bytestream);
    }

    // Determine CRC location relative to current payload end
    let crc_location = validate_crc_location(bytestream.len(), header)?;

    // Padding for CRC alignment
    if let CrcLocation::Keyword(_) = &header.crc_location {
        bytestream.resize(crc_location as usize, header.padding);
    }

    // Fill whole block if the CRC area is block
    if settings.crc.area == CrcArea::Block {
        bytestream.resize(header.length as usize, header.padding);
        bytestream[crc_location as usize..(crc_location + 4) as usize].fill(0);
    }

    // Compute CRC based on selected area
    let crc_val = checksum::calculate_crc(bytestream);

    let mut crc_bytes = match settings.endianness {
        Endianness::Big => crc_val.to_be_bytes(),
        Endianness::Little => crc_val.to_le_bytes(),
    };
    if byte_swap {
        byte_swap_inplace(&mut crc_bytes);
    }

    // Resize to full block if pad_to_end is true
    if pad_to_end {
        bytestream.resize(header.length as usize, header.padding);
    }

    let hex_string = emit_hex(
        &[DataRange {
            start_address: header.start_address + settings.virtual_offset,
            bytestream: bytestream.as_slice(),
            crc_address: header.start_address + settings.virtual_offset + crc_location,
            crc_bytestream: &crc_bytes,
        }],
        record_width,
        format,
    )?;
    Ok(hex_string)
}

fn emit_hex<'a>(
    ranges: &[DataRange<'a>],
    record_width: usize,
    format: OutputFormat,
) -> Result<String, NvmError> {
    // Use bin_file to format output.
    let mut bf = BinFile::new();
    let mut max_end: usize = 0;

    for range in ranges {
        bf.add_bytes(range.bytestream, Some(range.start_address as usize), false)
            .map_err(|e| NvmError::HexOutputError(format!("Failed to add bytes: {}", e)))?;
        bf.add_bytes(range.crc_bytestream, Some(range.crc_address as usize), true)
            .map_err(|e| NvmError::HexOutputError(format!("Failed to add bytes: {}", e)))?;

        let end = (range.start_address as usize).saturating_add(range.bytestream.len());
        if end > max_end {
            max_end = end;
        }
        let end = (range.crc_address as usize).saturating_add(range.crc_bytestream.len());
        if end > max_end {
            max_end = end;
        }
    }

    match format {
        OutputFormat::Hex => {
            let ihex_format = if max_end <= 0x1_0000 {
                IHexFormat::IHex16
            } else {
                IHexFormat::IHex32
            };
            let lines = bf.to_ihex(Some(record_width), ihex_format).map_err(|e| {
                NvmError::HexOutputError(format!("Failed to generate Intel HEX: {}", e))
            })?;
            Ok(lines.join("\n"))
        }
        OutputFormat::Mot => {
            use bin_file::SRecordAddressLength;
            let addr_len = if max_end <= 0x1_0000 {
                SRecordAddressLength::Length16
            } else if max_end <= 0x100_0000 {
                SRecordAddressLength::Length24
            } else {
                SRecordAddressLength::Length32
            };
            let lines = bf.to_srec(Some(record_width), addr_len).map_err(|e| {
                NvmError::HexOutputError(format!("Failed to generate S-Record: {}", e))
            })?;
            Ok(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::header::CrcLocation;
    use crate::layout::header::Header;
    use crate::layout::settings::Endianness;
    use crate::layout::settings::Settings;
    use crate::layout::settings::{CrcArea, CrcData};

    fn sample_settings() -> Settings {
        Settings {
            endianness: Endianness::Little,
            virtual_offset: 0,
            crc: CrcData {
                polynomial: 0x04C11DB7,
                start: 0xFFFF_FFFF,
                xor_out: 0xFFFF_FFFF,
                ref_in: true,
                ref_out: true,
                area: CrcArea::Data,
            },
            byte_swap: false,
            pad_to_end: false,
        }
    }

    fn sample_header(len: u32) -> Header {
        Header {
            start_address: 0,
            length: len,
            crc_location: CrcLocation::Keyword("end".to_string()),
            padding: 0xFF,
        }
    }

    #[test]
    fn pad_to_end_false_resizes_to_crc_end_only() {
        let settings = sample_settings();
        checksum::init_crc_algorithm(&settings.crc);
        let header = sample_header(16);

        let mut bytestream = vec![1u8, 2, 3, 4];
        let hex = bytestream_to_hex_string(
            &mut bytestream,
            &header,
            &settings,
            false,
            16,
            false,
            crate::output::args::OutputFormat::Hex,
        )
        .expect("hex generation failed");

        // No in-memory resize when pad_to_end=false; CRC is emitted separately
        assert_eq!(bytestream.len(), 4);

        // And the emitted hex should contain the CRC bytes (endianness applied)
        let crc_location = super::validate_crc_location(4usize, &header).expect("crc loc");
        assert_eq!(crc_location as usize, 4, "crc should follow payload end");
        let crc_val = checksum::calculate_crc(&bytestream[..crc_location as usize]);
        let crc_bytes = match settings.endianness {
            Endianness::Big => crc_val.to_be_bytes(),
            Endianness::Little => crc_val.to_le_bytes(),
        };
        // No byte swap in this test
        let expected_crc_ascii = crc_bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();
        assert!(
            hex.to_uppercase().contains(&expected_crc_ascii),
            "hex should contain CRC bytes"
        );
    }
    #[test]
    fn pad_to_end_true_resizes_to_full_block() {
        let settings = sample_settings();
        checksum::init_crc_algorithm(&settings.crc);
        let header = sample_header(32);

        let mut bytestream = vec![1u8, 2, 3, 4];
        let _hex = bytestream_to_hex_string(
            &mut bytestream,
            &header,
            &settings,
            false,
            16,
            true,
            crate::output::args::OutputFormat::Hex,
        )
        .expect("hex generation failed");

        assert_eq!(bytestream.len(), header.length as usize);
    }
}
