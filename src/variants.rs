use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VariantError {
    #[error("Failed to read file")]
    FailedToReadFile,
    #[error("Failed to parse file")]
    FailedToParseFile,
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Row not found")]
    RowNotFound,
    #[error("Invalid cell")]
    InvalidCell,
    #[error("Array too long")]
    ArrayTooLong,
    #[error("Bad name")]
    BadName,
}

pub struct DataSheet {
    names: Vec<String>,
    default_values: Vec<Data>,
    debug_values: Option<Vec<Data>>,
    variant_values: Option<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl DataSheet {
    pub fn new(filename: &str, variant: Option<&str>, debug: bool) -> Result<Self, VariantError> {
        let mut workbook: Xlsx<_> =
            open_workbook(filename).map_err(|_| VariantError::FailedToReadFile)?;

        let main_sheet = workbook
            .worksheet_range("Main")
            .map_err(|_| VariantError::ColumnNotFound("Main".to_string()))?;

        let rows: Vec<_> = main_sheet.rows().collect();
        let data_rows = rows.len() - 1;
        let headers = &rows[0];

        let name_index = headers
            .iter()
            .position(|cell| cell.to_string() == "Name")
            .ok_or(VariantError::ColumnNotFound("Name".to_string()))?;

        let default_index = headers
            .iter()
            .position(|cell| cell.to_string() == "Default")
            .ok_or(VariantError::ColumnNotFound("Default".to_string()))?;

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| row[name_index].to_string()));

        let mut default_values: Vec<Data> = Vec::with_capacity(data_rows);
        default_values.extend(rows.iter().skip(1).map(|row| row[default_index].clone()));

        let mut debug_values: Option<Vec<Data>> = None;
        if debug {
            let debug_index = headers
                .iter()
                .position(|cell| cell.to_string() == "Debug")
                .ok_or(VariantError::ColumnNotFound("Debug".to_string()))?;

            let mut debug_vec: Vec<Data> = Vec::with_capacity(data_rows);
            debug_vec.extend(rows.iter().skip(1).map(|row| row[debug_index].clone()));

            debug_values = Some(debug_vec);
        }

        let mut variant_values: Option<Vec<Data>> = None;
        if let Some(ref name) = variant {
            let variant_index = headers
                .iter()
                .position(|cell| cell.to_string() == *name)
                .ok_or(VariantError::ColumnNotFound(name.to_string()))?;

            let mut variant_vec: Vec<Data> = Vec::with_capacity(data_rows);
            variant_vec.extend(rows.iter().skip(1).map(|row| row[variant_index].clone()));

            variant_values = Some(variant_vec);
        };

        let mut sheets: HashMap<String, Range<Data>> =
            HashMap::with_capacity(workbook.worksheets().len() - 1);
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

    pub fn retrieve_cell_data(&self, name: &str) -> Result<Data, VariantError> {
        let index = self
            .names
            .iter()
            .position(|n| n == name)
            .ok_or(VariantError::RowNotFound)?;

        if let Some(debug_values) = &self.debug_values {
            if let Some(debug) = debug_values.get(index) {
                if !matches!(debug, Data::Empty) {
                    return Ok(debug.clone());
                }
            }
        }

        if let Some(variant_values) = &self.variant_values {
            if let Some(variant) = variant_values.get(index) {
                if !matches!(variant, Data::Empty) {
                    return Ok(variant.clone());
                }
            }
        }

        if let Some(default) = self.default_values.get(index) {
            if !matches!(default, Data::Empty) {
                return Ok(default.clone());
            }
        }

        Err(VariantError::RowNotFound)
    }

    // TODO: retrieve sheets by name, data format to be decided
}
