use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use std::collections::HashMap;

use crate::error::*;
use crate::schema::*;

pub struct DataSheet {
    names: Vec<String>,
    // Lowercased name -> index for case-insensitive lookup from layout to Excel
    names_lower_to_index: HashMap<String, usize>,
    default_values: Vec<Data>,
    debug_values: Option<Vec<Data>>,
    variant_values: Option<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl DataSheet {
    pub fn new(filename: &str, variant: &Option<String>, debug: bool) -> Result<Self, NvmError> {
        let mut workbook: Xlsx<_> = open_workbook(filename)
            .map_err(|_| NvmError::FileError("failed to open file: ".to_string() + filename))?;

        let main_sheet = workbook
            .worksheet_range("Main")
            .map_err(|_| NvmError::MiscError("Main sheet not found.".to_string()))?;

        let rows: Vec<_> = main_sheet.rows().collect();
        let (headers, data_rows) = match rows.split_first() {
            Some((hdr, tail)) => (hdr, tail.len()),
            None => {
                return Err(NvmError::RetrievalError(
                    "invalid main sheet format.".to_string(),
                ));
            }
        };

        let name_index = headers
            .iter()
            .position(|cell| Self::cell_eq_ascii(cell, "Name"))
            .ok_or(NvmError::ColumnNotFound("Name".to_string()))?;

        // Allow a few aliases for the default column, case-insensitive
        let default_aliases = ["Default", "Defaults", "Generic"];
        let default_index = headers
            .iter()
            .position(|cell| default_aliases.iter().any(|opt| Self::cell_eq_ascii(cell, opt)))
            .ok_or(NvmError::ColumnNotFound("Default".to_string()))?;

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| row[name_index].to_string()));

        // Build a lowercase name -> index map for case-insensitive lookups
        let mut names_lower_to_index: HashMap<String, usize> = HashMap::with_capacity(names.len());
        for (idx, nm) in names.iter().enumerate() {
            names_lower_to_index.insert(nm.trim().to_ascii_lowercase(), idx);
        }

        let mut default_values: Vec<Data> = Vec::with_capacity(data_rows);
        default_values.extend(rows.iter().skip(1).map(|row| row[default_index].clone()));

        let mut debug_values: Option<Vec<Data>> = None;
        if debug {
            let debug_index = headers
                .iter()
                .position(|cell| Self::cell_eq_ascii(cell, "Debug"))
                .ok_or(NvmError::ColumnNotFound("Debug".to_string()))?;

            let mut debug_vec: Vec<Data> = Vec::with_capacity(data_rows);
            debug_vec.extend(rows.iter().skip(1).map(|row| row[debug_index].clone()));

            debug_values = Some(debug_vec);
        }

        let mut variant_values: Option<Vec<Data>> = None;
        if let Some(name) = variant {
            let variant_index = headers
                .iter()
                .position(|cell| Self::cell_eq_ascii(cell, name))
                .ok_or(NvmError::ColumnNotFound(name.to_string()))?;

            let mut variant_vec: Vec<Data> = Vec::with_capacity(data_rows);
            variant_vec.extend(rows.iter().skip(1).map(|row| row[variant_index].clone()));

            variant_values = Some(variant_vec);
        };

        let mut sheets: HashMap<String, Range<Data>> =
            HashMap::with_capacity(workbook.worksheets().len().saturating_sub(1));
        for (name, sheet) in workbook.worksheets() {
            if name != "Main" {
                sheets.insert(name.to_string(), sheet);
            }
        }

        Ok(Self {
            names,
            names_lower_to_index,
            default_values,
            debug_values,
            variant_values,
            sheets,
        })
    }

    pub fn retrieve_single_value(&self, name: &str) -> Result<DataValue, NvmError> {
        match self.retrieve_cell(name)? {
            Data::Int(i) => Ok(DataValue::I64(*i)),
            Data::Float(f) => Ok(DataValue::F64(*f)),
            _ => Err(NvmError::RetrievalError(
                "Found non-numeric single value: ".to_string() + name,
            )),
        }
    }

    pub fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, NvmError> {
        let Data::String(cell_string) = self.retrieve_cell(name)? else {
            return Err(NvmError::RetrievalError(
                "Expected string value for 1D array or string: ".to_string() + name,
            ));
        };

        // If we find a sheet, we assume it's a 1D array
        if let Some(sheet) = self.sheets.get(cell_string) {
            let mut out = Vec::new();

            for row in sheet.rows().skip(1) {
                match row.first() {
                    Some(cell) if !Self::cell_is_empty(cell) => {
                        let v = match cell {
                            Data::Int(i) => DataValue::I64(*i),
                            Data::Float(f) => DataValue::F64(*f),
                            Data::String(s) => DataValue::Str(s.to_owned()),
                            _ => {
                                return Err(NvmError::RetrievalError(
                                    "Unsupported data type in 1D array: ".to_string() + name,
                                ));
                            }
                        };
                        out.push(v);
                    }
                    _ => break,
                }
            }
            return Ok(ValueSource::Array(out));
        }

        // We don't find a sheet, so we assume it's a string
        Ok(ValueSource::Single(DataValue::Str(cell_string.to_owned())))
    }

    pub fn retrieve_2d_array(&self, name: &str) -> Result<Vec<Vec<DataValue>>, NvmError> {
        let Data::String(cell_string) = self.retrieve_cell(name)? else {
            return Err(NvmError::RetrievalError(
                "Expected string value for 2D array: ".to_string() + name,
            ));
        };

        let sheet = self.sheets.get(cell_string).ok_or_else(|| {
            NvmError::RetrievalError("Sheet not found: ".to_string() + cell_string)
        })?;

        let convert = |cell: &Data| -> Result<DataValue, NvmError> {
            match cell {
                Data::Int(i) => Ok(DataValue::I64(*i)),
                Data::Float(f) => Ok(DataValue::F64(*f)),
                _ => Err(NvmError::RetrievalError(
                    "Unsupported data type in 2D array: ".to_string() + name,
                )),
            }
        };

        let mut rows = sheet.rows();
        let hdrs = rows.next().ok_or_else(|| {
            NvmError::RetrievalError("No headers found in 2D array: ".to_string() + name)
        })?;
        let width = hdrs.iter().take_while(|c| !Self::cell_is_empty(c)).count();
        if width == 0 {
            return Err(NvmError::RetrievalError(
                "Detected zero width 2D array: ".to_string() + name,
            ));
        }

        let mut out = Vec::new();

        'outer: for row in rows {
            if row.first().is_none_or(Self::cell_is_empty) {
                break;
            }

            let mut vals = Vec::with_capacity(width);
            for col in 0..width {
                let Some(cell) = row.get(col) else {
                    break 'outer;
                };
                if Self::cell_is_empty(cell) {
                    break 'outer;
                };
                vals.push(convert(cell)?);
            }
            out.push(vals);
        }

        Ok(out)
    }

    fn retrieve_cell(&self, name: &str) -> Result<&Data, NvmError> {
        let key = name.trim().to_ascii_lowercase();
        let index = *self
            .names_lower_to_index
            .get(&key)
            .ok_or(NvmError::RetrievalError(
                "index not found for ".to_string() + name,
            ))?;

        if let Some(v) = [
            self.debug_values.as_ref().and_then(|v| v.get(index)),
            self.variant_values.as_ref().and_then(|v| v.get(index)),
            self.default_values.get(index),
        ]
        .iter()
        .flatten()
        .find(|d| !Self::cell_is_empty(d))
        {
            return Ok(v);
        }

        Err(NvmError::RetrievalError(
            "data not found for ".to_string() + name,
        ))
    }

    fn cell_eq_ascii(cell: &Data, target: &str) -> bool {
        match cell {
            Data::String(s) => s.trim().eq_ignore_ascii_case(target),
            _ => false,
        }
    }

    fn cell_is_empty(cell: &Data) -> bool {
        match cell {
            Data::Empty => true,
            Data::String(s) => s.trim().is_empty(),
            _ => false,
        }
    }

    // TODO: retrieve sheets by name, data format to be decided
}
