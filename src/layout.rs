use crate::error::*;
use crate::schema::*;
use crate::variants::DataSheet;
use std::path::Path;

impl LeafEntry {
    /// Returns the alignment of the leaf entry.
    pub fn get_alignment(&self) -> usize {
        self.scalar_type.size_bytes()
    }

    pub fn emit_bytes(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
        padding: &u8,
    ) -> Result<Vec<u8>, NvmError> {
        match self.size {
            None => self.emit_bytes_single(data_sheet, endianness),
            Some(SizeSource::OneD(size)) => {
                let bytes = self.emit_bytes_1d(data_sheet, endianness, size, padding)?;
                Ok(bytes)
            }
            Some(SizeSource::TwoD(size)) => {
                let bytes = self.emit_bytes_2d(data_sheet, endianness, size, padding)?;
                Ok(bytes)
            }
        }
    }

    fn emit_bytes_single(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
    ) -> Result<Vec<u8>, NvmError> {
        match &self.source {
            EntrySource::Name(name) => {
                let value = data_sheet.retrieve_single_value(name)?;
                value.to_bytes(self.scalar_type, endianness)
            }
            EntrySource::Value(ValueSource::Single(v)) => v.to_bytes(self.scalar_type, endianness),
            EntrySource::Value(_) => Err(NvmError::DataValueExportFailed(
                "Single value expected for scalar type.".to_string(),
            )),
        }
    }

    fn emit_bytes_1d(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
        size: usize,
        padding: &u8,
    ) -> Result<Vec<u8>, NvmError> {
        let mut out = Vec::with_capacity(size * self.scalar_type.size_bytes());

        match &self.source {
            EntrySource::Name(name) => match data_sheet.retrieve_1d_array_or_string(name)? {
                ValueSource::Single(v) => {
                    if !matches!(self.scalar_type, ScalarType::U8) {
                        return Err(NvmError::DataValueExportFailed(
                            "Strings should have type u8.".to_string(),
                        ));
                    }
                    out.extend(v.string_to_bytes()?);
                }
                ValueSource::Array(v) => {
                    for v in v {
                        out.extend(v.to_bytes(self.scalar_type, endianness)?);
                    }
                }
            },
            EntrySource::Value(ValueSource::Array(v)) => {
                for v in v {
                    out.extend(v.to_bytes(self.scalar_type, endianness)?);
                }
            }
            EntrySource::Value(ValueSource::Single(v)) => {
                if !matches!(self.scalar_type, ScalarType::U8) {
                    return Err(NvmError::DataValueExportFailed(
                        "Strings should have type u8.".to_string(),
                    ));
                }
                out.extend(v.string_to_bytes()?);
            }
        }

        if out.len() > (size * self.scalar_type.size_bytes()) {
            return Err(NvmError::DataValueExportFailed(
                "Array/string is larger than defined size.".to_string(),
            ));
        }
        while out.len() < (size * self.scalar_type.size_bytes()) {
            out.push(*padding);
        }
        Ok(out)
    }

    fn emit_bytes_2d(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
        size: [usize; 2],
        padding: &u8,
    ) -> Result<Vec<u8>, NvmError> {
        match &self.source {
            EntrySource::Name(name) => {
                let data = data_sheet.retrieve_2d_array(name)?;

                let rows = size[0];
                let cols = size[1];

                let total_bytes = rows * cols * self.scalar_type.size_bytes();

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

                let mut out = Vec::with_capacity(total_bytes);
                for row in data {
                    for v in row {
                        out.extend(v.to_bytes(self.scalar_type, endianness)?);
                    }
                }

                while out.len() < total_bytes {
                    out.push(*padding);
                }

                Ok(out)
            }
            EntrySource::Value(_) => Err(NvmError::DataValueExportFailed(
                "2D arrays within the layout file are not supported.".to_string(),
            )),
        }
    }
}

impl Block {
    pub fn build_bytestream(
        &self,
        data_sheet: &DataSheet,
        settings: &Settings,
    ) -> Result<Vec<u8>, NvmError> {
        let mut buffer = Vec::with_capacity(self.header.length as usize);
        let mut offset = 0;

        Self::build_bytestream_inner(
            &self.data,
            data_sheet,
            &mut buffer,
            &mut offset,
            &settings.endianness,
            &self.header.padding,
        )?;

        if matches!(self.header.crc_location, CrcLocation::Keyword(_)) {
            // Padding out to the 4 byte boundary for appended/prepended CRC32
            while offset % 4 != 0 {
                buffer.push(self.header.padding);
                offset += 1;
            }
        }

        Ok(buffer)
    }

    fn build_bytestream_inner(
        table: &Entry,
        data_sheet: &DataSheet,
        buffer: &mut Vec<u8>,
        offset: &mut usize,
        endianness: &Endianness,
        padding: &u8,
    ) -> Result<(), NvmError> {
        match table {
            Entry::Leaf(leaf) => {
                let alignment = leaf.get_alignment();
                while *offset % alignment != 0 {
                    buffer.push(*padding);
                    *offset += 1;
                }

                let bytes = leaf.emit_bytes(data_sheet, endianness, padding)?;
                *offset += bytes.len();
                buffer.extend(bytes);
            }
            Entry::Branch(branch) => {
                for (_, v) in branch.iter() {
                    Self::build_bytestream_inner(
                        v, data_sheet, buffer, offset, endianness, padding,
                    )?;
                }
            }
        }
        Ok(())
    }
}

pub fn load_layout(filename: &str) -> Result<Config, NvmError> {
    let text = std::fs::read_to_string(filename)
        .map_err(|_| NvmError::FileError(format!("failed to open file: {}", filename)))?;

    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let cfg: Config = match ext.as_str() {
        "toml" => toml::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "yaml" | "yml" => serde_yaml::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        "json" => serde_json::from_str(&text).map_err(|e| {
            NvmError::FileError(format!("failed to parse file {}: {}", filename, e))
        })?,
        _ => return Err(NvmError::FileError("Unsupported file format".to_string())),
    };

    Ok(cfg)
}
