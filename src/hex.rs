use crate::error::*;
use crate::schema::*;

use crc::{Algorithm, Crc};
use ihex::{Record, create_object_file_representation};

fn byte_swap_inplace(bytes: &mut [u8]) {
    for chunk in bytes.chunks_exact_mut(2) {
        chunk.swap(0, 1);
    }
}

fn calculate_crc(bytestream: &[u8], crc_settings: &CrcData) -> u32 {
    let crc_algo = Algorithm::<u32> {
        width: 32,
        poly: crc_settings.polynomial,
        init: crc_settings.start,
        refin: crc_settings.ref_in,
        refout: crc_settings.ref_out,
        xorout: crc_settings.xor_out,
        check: 0,
        residue: 0,
    };
    let algo_static: &'static Algorithm<u32> = Box::leak(Box::new(crc_algo));

    let crc_calc = Crc::<u32>::new(algo_static);
    let mut crc_digest = crc_calc.digest();
    crc_digest.update(bytestream);
    crc_digest.finalize()
}

pub fn bytestream_to_hex_string(
    bytestream: &mut Vec<u8>,
    header: &Header,
    settings: &Settings,
    offset: u32,
    byte_swap: bool,
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

    let crc_val = calculate_crc(bytestream, &settings.crc);

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

    bytestream.resize(header.length as usize, header.padding);
    bytestream[crc_offset as usize..(crc_offset + 4) as usize].copy_from_slice(&crc_bytes);

    let hex_string = emit_hex(header.start_address + offset, bytestream)?;
    Ok(hex_string)
}

fn emit_hex(start_address: u32, bytestream: &[u8]) -> Result<String, NvmError> {
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
        let n = (bytestream.len() - idx).min(32).min(seg_rem);

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
