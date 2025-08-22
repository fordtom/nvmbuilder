use crate::error::*;
use crate::schema::*;
use crate::variants::DataSheet;

impl LeafEntry {
    /// Returns the alignment of the leaf entry.
    pub fn get_alignment(&self) -> usize {
        self.scalar_type.size_bytes()
    }

    pub fn emit_bytes(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
        padding: u8,
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
        padding: u8,
    ) -> Result<Vec<u8>, NvmError> {
        let mut out = Vec::with_capacity(size * self.scalar_type.size_bytes());

        match &self.source {
            EntrySource::Name(name) => match data_sheet.retrieve_1d_array_or_string(name)? {
                ValueSource::Single(v) => {
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
                out.extend(v.string_to_bytes()?);
            }
        }

        if out.len() > (size * self.scalar_type.size_bytes()) {
            return Err(NvmError::DataValueExportFailed(
                "Array/string is larger than defined size.".to_string(),
            ));
        }
        while out.len() < (size * self.scalar_type.size_bytes()) {
            out.push(padding);
        }
        Ok(out)
    }

    fn emit_bytes_2d(
        &self,
        data_sheet: &DataSheet,
        endianness: &Endianness,
        size: [usize; 2],
        padding: u8,
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
                    out.push(padding);
                }

                Ok(out)
            }
            EntrySource::Value(_) => {
                return Err(NvmError::DataValueExportFailed(
                    "2D arrays within the layout file are not supported.".to_string(),
                ));
            }
        }
    }
}
