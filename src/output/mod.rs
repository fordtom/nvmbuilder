pub mod args;
pub mod checksum;

use crate::error::*;
use crate::layout::header::{CrcLocation, Header};
use crate::layout::settings::{Endianness, Settings};
use crate::output::args::OutputFormat;

use bin_file::{BinFile, IHexFormat};

#[derive(Debug, Clone)]
pub struct DataRange<'a> {
    pub start_address: u32,
    pub bytestream: &'a [u8],
}

fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
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

    let crc_val = checksum::calculate_crc(bytestream);

    let mut crc_bytes = match settings.endianness {
        Endianness::Big => crc_val.to_be_bytes(),
        Endianness::Little => crc_val.to_le_bytes(),
    };
    if byte_swap {
        byte_swap_inplace(&mut crc_bytes);
    }

    let crc_offset = match &header.crc_location {
        CrcLocation::Address(address) => {
            let crc_offset = address.checked_sub(header.start_address).ok_or_else(|| {
                NvmError::HexOutputError("CRC address before block start.".to_string())
            })?;

            if crc_offset < bytestream.len() as u32 {
                return Err(NvmError::HexOutputError(
                    "CRC overlaps with payload.".to_string(),
                ));
            }

            crc_offset
        }
        CrcLocation::Keyword(option) => match option.as_str() {
            "end" => bytestream.len() as u32,
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

    let min_len = (crc_offset + 4) as usize;
    let target_len = if pad_to_end {
        header.length as usize
    } else {
        min_len
    };
    bytestream.resize(target_len, header.padding);
    bytestream[crc_offset as usize..(crc_offset + 4) as usize].copy_from_slice(&crc_bytes);

    let hex_string = emit_hex(
        &[DataRange {
            start_address: header.start_address + settings.virtual_offset,
            bytestream: bytestream.as_slice(),
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
    for range in ranges {
        bf.add_bytes(range.bytestream, Some(range.start_address as usize), false)
            .map_err(|e| NvmError::HexOutputError(format!("Failed to add bytes: {}", e)))?;
    }

    match format {
        OutputFormat::Hex => {
            // Select format based on the highest end address across ranges
            let mut max_end: usize = 0;
            for range in ranges {
                let end = (range.start_address as usize).saturating_add(range.bytestream.len());
                if end > max_end {
                    max_end = end;
                }
            }
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
            // Auto-select SREC address length based on range, mimicking IHex selection
            let mut max_end: usize = 0;
            for range in ranges {
                let end = (range.start_address as usize).saturating_add(range.bytestream.len());
                if end > max_end {
                    max_end = end;
                }
            }
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
    use crate::layout::settings::CrcData;
    use crate::layout::settings::Endianness;
    use crate::layout::settings::Settings;

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
        let _hex = bytestream_to_hex_string(
            &mut bytestream,
            &header,
            &settings,
            false,
            16,
            false,
            crate::output::args::OutputFormat::Hex,
        )
        .expect("hex generation failed");

        // 4 bytes payload + 4 bytes CRC
        assert_eq!(bytestream.len(), 8);
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
