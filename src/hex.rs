use crate::error::*;
use crate::schema::*;

use crc::{Algorithm, Crc};
use ihex::{create_object_file_representation, Record};

pub fn bytestream_to_hex_string(
    bytestream: &mut Vec<u8>,
    header: &Header,
    settings: &Settings,
    offset: u32,
) -> Result<String, NvmError> {
    if bytestream.len() > header.length as usize {
        return Err(NvmError::HexOutputError(
            "Bytestream length exceeds block length.".to_string(),
        ));
    }

    let crc_offset = header
        .crc_location
        .checked_sub(header.start_address)
        .ok_or_else(|| NvmError::HexOutputError("CRC before block start.".to_string()))?;

    if crc_offset < bytestream.len() as u32 {
        return Err(NvmError::HexOutputError(
            "CRC overlaps with payload.".to_string(),
        ));
    }

    let remaining_space = header.length.checked_sub(crc_offset).ok_or_else(|| {
        NvmError::HexOutputError("CRC location is beyond block length.".to_string())
    })?;
    if remaining_space < 4 {
        return Err(NvmError::HexOutputError(
            "CRC location would overrun block.".to_string(),
        ));
    }

    bytestream.resize(header.length as usize, header.padding);

    let crc_offset = header.crc_location - header.start_address;
    bytestream[crc_offset as usize..(crc_offset + 4) as usize].fill(0);

    let crc_algo = Algorithm::<u32> {
        width: 32,
        poly: settings.crc.polynomial,
        init: settings.crc.start,
        refin: false,
        refout: settings.crc.reverse,
        xorout: settings.crc.xor_out,
        check: 0,
        residue: 0,
    };
    let algo_static: &'static Algorithm<u32> = Box::leak(Box::new(crc_algo));

    let crc_calc = Crc::<u32>::new(algo_static);
    let mut crc_digest = crc_calc.digest();
    crc_digest.update(&bytestream);
    let crc_val = crc_digest.finalize();

    let crc_bytes = match settings.endianness {
        Endianness::Big => crc_val.to_be_bytes(),
        Endianness::Little => crc_val.to_le_bytes(),
    };
    bytestream[crc_offset as usize..(crc_offset + 4) as usize].copy_from_slice(&crc_bytes);

    let hex_string = emit_hex(header.start_address + offset, &bytestream)?;
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
