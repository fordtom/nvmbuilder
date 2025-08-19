use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::HashMap;

use crate::error::*;
use crate::types::*;

pub struct DataSheet {
    names: Vec<String>,
    default_values: Vec<Data>,
    debug_values: Option<Vec<Data>>,
    variant_values: Option<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl DataSheet {
    pub fn new(filename: &str, variant: Option<&str>, debug: bool) -> Result<Self, NvmError> {
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

        let default_index = headers
            .iter()
            .position(|cell| Self::cell_eq_ascii(cell, "Default"))
            .ok_or(NvmError::ColumnNotFound("Default".to_string()))?;

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| row[name_index].to_string()));

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
        if let Some(ref name) = variant {
            let variant_index = headers
                .iter()
                .position(|cell| cell.to_string() == *name)
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
            default_values,
            debug_values,
            variant_values,
            sheets,
        })
    }

    pub fn retrieve_single_value(&self, name: &str) -> Result<DataValue, NvmError> {
        let data = self.retrieve_cell(name)?;
        match data {
            Data::Int(i) => Ok(DataValue::I64(*i)),
            Data::Float(f) => Ok(DataValue::F64(*f)),
            _ => Err(NvmError::RetrievalError(
                "Found non-numeric single value: ".to_string() + name,
            )),
        }
    }

    pub fn retrieve_1d_array_or_string(&self, name: &str) -> Result<ValueSource, NvmError> {
        let data = self.retrieve_cell(name)?;

        let cell_string = if let Data::String(s) = data {
            s
        } else {
            return Err(NvmError::RetrievalError(
                "Expected string value for 1D array or string: ".to_string() + name,
            ));
        };

        if self.sheets.contains_key(cell_string) {
            let sheet = self
                .sheets
                .get(cell_string)
                .ok_or(NvmError::RetrievalError(
                    "Sheet not found: ".to_string() + &cell_string,
                ))?;
        }

        // let mut values = Vec::with_capacity(arr.len());
        // for cell in arr {
        //     match cell {
        //         Data::Int(i) => values.push(DataValue::I64(i)),
        //         Data::Float(f) => values.push(DataValue::F64(f)),
        //         _ => {
        //             return Err(NvmError::RetrievalError(
        //                 "Found non-numeric value in 1D array: ".to_string() + name,
        //             ))
        //         }
        //     }
        // }
        // Ok(ValueSource::Array { value: values })
        // }
        // _ => Err(NvmError::RetrievalError(
        //     "Found non-numeric 1D array or string: ".to_string() + name,
        // )),

        Ok(ValueSource::Single(DataValue::Str(cell_string.to_owned())))
    }

    // pub fn retrieve_2d_array(&self, name: &str) -> Result<something, NvmError> {}

    fn retrieve_cell(&self, name: &str) -> Result<&Data, NvmError> {
        let index = self
            .names
            .iter()
            .position(|n| n == name)
            .ok_or(NvmError::RetrievalError(
                "index not found for ".to_string() + name,
            ))?;

        if let Some(debug_values) = &self.debug_values {
            if let Some(debug) = debug_values.get(index) {
                if !Self::cell_is_empty(debug) {
                    return Ok(debug);
                }
            }
        }

        if let Some(variant_values) = &self.variant_values {
            if let Some(variant) = variant_values.get(index) {
                if !Self::cell_is_empty(variant) {
                    return Ok(variant);
                }
            }
        }

        if let Some(default) = self.default_values.get(index) {
            if !Self::cell_is_empty(default) {
                return Ok(default);
            }
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
