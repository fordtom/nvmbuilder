pub mod checksum;
pub mod args;

use crate::error::*;
use crate::layout::header::{CrcLocation, Header};
use crate::layout::settings::{Endianness, Settings};

use ihex::{Record, create_object_file_representation};

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
        header.start_address + settings.virtual_offset,
        bytestream,
        record_width,
    )?;
    Ok(hex_string)
}

fn emit_hex(
    start_address: u32,
    bytestream: &[u8],
    record_width: usize,
) -> Result<String, NvmError> {
    let mut records = Vec::<Record>::new();
    let mut addr = start_address;
    let mut idx = 0usize;
    let mut upper: Option<u16> = None;

    while idx < bytestream.len() {
        let hi = (addr >> 16) as u16;
        if upper != Some(hi) {
            if hi != 0 {
                records.push(Record::ExtendedLinearAddress(hi));
            }
            upper = Some(hi);
        }

        let seg_rem = (0x1_0000 - (addr & 0xFFFF)) as usize;
        let n = (bytestream.len() - idx).min(record_width).min(seg_rem);

        records.push(Record::Data {
            offset: (addr & 0xFFFF) as u16,
            value: bytestream[idx..idx + n].to_vec(),
        });

        idx += n;
        addr += n as u32;
    }

    records.push(Record::EndOfFile);
    let obj = create_object_file_representation(&records).map_err(|_| {
        NvmError::HexOutputError("Failed to create object file representation.".to_string())
    })?;
    Ok(obj)
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
        let _hex = bytestream_to_hex_string(&mut bytestream, &header, &settings, false, 16, false)
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
        let _hex = bytestream_to_hex_string(&mut bytestream, &header, &settings, false, 16, true)
            .expect("hex generation failed");

        assert_eq!(bytestream.len(), header.length as usize);
    }
}
