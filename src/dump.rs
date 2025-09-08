use crate::error::*;
use crate::schema::*;
use crate::variants::DataSheet;

use serde_json::{json, Map, Value};

pub struct DumpResult {
    pub values: Value,
    pub meta: Value,
}

pub fn build_dump(
    block: &Block,
    data_sheet: &DataSheet,
    settings: &Settings,
) -> Result<DumpResult, NvmError> {
    let mut offset: usize = 0;
    let mut padding_meta: Map<String, Value> = Map::new();

    let values = walk_entry(
        "",
        &block.data,
        data_sheet,
        &settings.endianness,
        block.header.padding,
        &mut offset,
        &mut padding_meta,
    )?;

    let mut meta_obj = Map::new();
    // header context
    let header = json!({
        "start_address": block.header.start_address,
        "length": block.header.length,
        "endianness": match settings.endianness { Endianness::Little => "little", Endianness::Big => "big" },
        "padding_byte": block.header.padding,
        "crc_location": match &block.header.crc_location {
            CrcLocation::Keyword(s) => json!({"keyword": s}),
            CrcLocation::Address(a) => json!({"address": a}),
        }
    });
    meta_obj.insert("header".to_string(), header);

    // padding details
    meta_obj.insert("padding".to_string(), Value::Object(padding_meta));

    // trailing padding before CRC (for keyword-based CRC placement)
    if matches!(block.header.crc_location, CrcLocation::Keyword(_)) {
        let mut crc_align = 0usize;
        while offset % 4 != 0 { crc_align += 1; offset += 1; }
        if crc_align != 0 {
            meta_obj.insert("crc_align_padding_bytes".to_string(), json!(crc_align));
        }
    }

    Ok(DumpResult {
        values,
        meta: Value::Object(meta_obj),
    })
}

fn walk_entry(
    path: &str,
    entry: &Entry,
    data_sheet: &DataSheet,
    endianness: &Endianness,
    padding_byte: u8,
    offset: &mut usize,
    padding_meta: &mut Map<String, Value>,
) -> Result<Value, NvmError> {
    match entry {
        Entry::Branch(branch) => {
            let mut out = Map::new();
            for (key, child) in branch.iter() {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{key}", path)
                };
                let v = walk_entry(
                    &child_path,
                    child,
                    data_sheet,
                    endianness,
                    padding_byte,
                    offset,
                    padding_meta,
                )?;
                out.insert(key.clone(), v);
            }
            Ok(Value::Object(out))
        }
        Entry::Leaf(leaf) => resolve_leaf(path, leaf, data_sheet, endianness, padding_byte, offset, padding_meta),
    }
}

fn resolve_leaf(
    path: &str,
    leaf: &LeafEntry,
    data_sheet: &DataSheet,
    endianness: &Endianness,
    padding_byte: u8,
    offset: &mut usize,
    padding_meta: &mut Map<String, Value>,
) -> Result<Value, NvmError> {
    let alignment = leaf.get_alignment();
    let pad_before = (alignment - (*offset % alignment)) % alignment;
    *offset += pad_before;

    let (value, consumed_bytes, within_bytes) = match leaf.size {
        None => resolve_scalar_value(leaf, data_sheet, endianness)?,
        Some(SizeSource::OneD(size)) => resolve_1d_value(leaf, data_sheet, endianness, padding_byte, size)?,
        Some(SizeSource::TwoD(dim)) => resolve_2d_value(leaf, data_sheet, endianness, padding_byte, dim)?,
    };

    *offset += consumed_bytes;

    let mut meta = Map::new();
    if pad_before != 0 {
        meta.insert("before".to_string(), json!(pad_before));
    }
    if within_bytes != 0 {
        meta.insert("within_bytes".to_string(), json!(within_bytes));
    }
    if !meta.is_empty() {
        padding_meta.insert(path.to_string(), Value::Object(meta));
    }

    Ok(value)
}

fn resolve_scalar_value(
    leaf: &LeafEntry,
    data_sheet: &DataSheet,
    endianness: &Endianness,
) -> Result<(Value, usize, usize), NvmError> {
    let v = match &leaf.source {
        EntrySource::Name(name) => data_sheet.retrieve_single_value(name)?,
        EntrySource::Value(ValueSource::Single(v)) => v.clone(),
        EntrySource::Value(ValueSource::Array(_)) => {
            return Err(NvmError::DataValueExportFailed(
                "Single value expected for scalar type.".to_string(),
            ))
        }
    };

    let value_json = data_value_to_json(&v, leaf.scalar_type)?;
    let bytes = leaf.scalar_type.size_bytes();
    Ok((value_json, bytes, 0))
}

fn resolve_1d_value(
    leaf: &LeafEntry,
    data_sheet: &DataSheet,
    endianness: &Endianness,
    padding_byte: u8,
    size: usize,
) -> Result<(Value, usize, usize), NvmError> {
    let scalar_size = leaf.scalar_type.size_bytes();
    let declared_bytes = size * scalar_size;

    match &leaf.source {
        EntrySource::Name(name) => match data_sheet.retrieve_1d_array_or_string(name)? {
            ValueSource::Single(v) => {
                if !matches!(leaf.scalar_type, ScalarType::U8) {
                    return Err(NvmError::DataValueExportFailed(
                        "Strings should have type u8.".to_string(),
                    ));
                }
                let s = match v {
                    DataValue::Str(ref s) => s.clone(),
                    _ => return Err(NvmError::DataValueExportFailed("String expected".to_string())),
                };
                let used_bytes = s.as_bytes().len();
                if used_bytes > declared_bytes {
                    return Err(NvmError::DataValueExportFailed(
                        "Array/string is larger than defined size.".to_string(),
                    ));
                }
                let within_bytes = declared_bytes - used_bytes;
                Ok((json!(s), declared_bytes, within_bytes))
            }
            ValueSource::Array(arr) => {
                let mut out = Vec::<Value>::with_capacity(arr.len());
                for dv in arr.iter() {
                    out.push(data_value_to_json(dv, leaf.scalar_type)?);
                }
                let used_elems = out.len();
                if used_elems > size {
                    return Err(NvmError::DataValueExportFailed(
                        "Array/string is larger than defined size.".to_string(),
                    ));
                }
                let within_bytes = (size - used_elems) * scalar_size;
                Ok((Value::Array(out), declared_bytes, within_bytes))
            }
        },
        EntrySource::Value(vs) => match vs {
            ValueSource::Array(arr) => {
                let mut out = Vec::<Value>::with_capacity(arr.len());
                for dv in arr.iter() {
                    out.push(data_value_to_json(dv, leaf.scalar_type)?);
                }
                let used_elems = out.len();
                if used_elems > size {
                    return Err(NvmError::DataValueExportFailed(
                        "Array/string is larger than defined size.".to_string(),
                    ));
                }
                let within_bytes = (size - used_elems) * scalar_size;
                Ok((Value::Array(out), declared_bytes, within_bytes))
            }
            ValueSource::Single(v) => {
                if !matches!(leaf.scalar_type, ScalarType::U8) {
                    return Err(NvmError::DataValueExportFailed(
                        "Strings should have type u8.".to_string(),
                    ));
                }
                let s = match v {
                    DataValue::Str(ref s) => s.clone(),
                    _ => return Err(NvmError::DataValueExportFailed("String expected".to_string())),
                };
                let used_bytes = s.as_bytes().len();
                if used_bytes > declared_bytes {
                    return Err(NvmError::DataValueExportFailed(
                        "Array/string is larger than defined size.".to_string(),
                    ));
                }
                let within_bytes = declared_bytes - used_bytes;
                Ok((json!(s), declared_bytes, within_bytes))
            }
        },
    }
}

fn resolve_2d_value(
    leaf: &LeafEntry,
    data_sheet: &DataSheet,
    endianness: &Endianness,
    padding_byte: u8,
    size: [usize; 2],
) -> Result<(Value, usize, usize), NvmError> {
    match &leaf.source {
        EntrySource::Name(name) => {
            let data = data_sheet.retrieve_2d_array(name)?;
            let rows = size[0];
            let cols = size[1];
            let total_bytes = rows * cols * leaf.scalar_type.size_bytes();

            if data.iter().any(|row| row.len() != cols) {
                return Err(NvmError::DataValueExportFailed(
                    "2D array column count mismatch.".to_string(),
                ));
            }

            if data.len() > rows {
                return Err(NvmError::DataValueExportFailed(
                    "2D array row count greater than defined size.".to_string(),
                ));
            }

            let mut out_rows: Vec<Value> = Vec::with_capacity(data.len());
            for row in data {
                let mut out_row: Vec<Value> = Vec::with_capacity(row.len());
                for dv in row {
                    out_row.push(data_value_to_json(&dv, leaf.scalar_type)?);
                }
                out_rows.push(Value::Array(out_row));
            }

            let used_rows = out_rows.len();
            let used_bytes = used_rows * cols * leaf.scalar_type.size_bytes();
            let within_bytes = total_bytes - used_bytes;
            Ok((Value::Array(out_rows), total_bytes, within_bytes))
        }
        EntrySource::Value(_) => Err(NvmError::DataValueExportFailed(
            "2D arrays within the layout file are not supported.".to_string(),
        )),
    }
}

fn data_value_to_json(value: &DataValue, scalar_type: ScalarType) -> Result<Value, NvmError> {
    let v = match scalar_type {
        ScalarType::U8 => json!(u8::try_from(value)?),
        ScalarType::I8 => json!(i8::try_from(value)?),
        ScalarType::U16 => json!(u16::try_from(value)?),
        ScalarType::I16 => json!(i16::try_from(value)?),
        ScalarType::U32 => json!(u32::try_from(value)?),
        ScalarType::I32 => json!(i32::try_from(value)?),
        ScalarType::U64 => json!(u64::try_from(value)?),
        ScalarType::I64 => json!(i64::try_from(value)?),
        ScalarType::F32 => json!(f32::try_from(value)?),
        ScalarType::F64 => json!(f64::try_from(value)?),
    };
    Ok(v)
}

