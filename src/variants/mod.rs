use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use std::collections::HashMap;

pub enum VariantError {
    FailedToReadFile,
    FailedToParseFile,
    NameColumnNotFound,
    DefaultColumnNotFound,
    OptionalColumnNotFound,
    RowNotFound,
    InvalidCell,
    ArrayTooLong,
}

pub struct DataSheet {
    names: Vec<String>,
    defaults: Vec<Data>,
    debugs: Option<Vec<Data>>,
    variants: Option<Vec<Data>>,
    sheets: HashMap<String, Range<Data>>,
}

impl DataSheet {
    pub fn new(
        filename: String,
        variant: Option<String>,
        debug: bool,
    ) -> Result<Self, VariantError> {
        let mut workbook: Xlsx<_> =
            open_workbook(filename).map_err(|_| VariantError::FailedToReadFile)?;

        let main_sheet = workbook
            .worksheet_range("Main")
            .map_err(|_| VariantError::DefaultColumnNotFound)?;

        let rows: Vec<_> = main_sheet.rows().collect();
        let data_rows = rows.len() - 1;
        let headers = &rows[0];

        let name_index = headers
            .iter()
            .position(|cell| cell.to_string() == "Name")
            .ok_or(VariantError::NameColumnNotFound)?;

        let default_index = headers
            .iter()
            .position(|cell| cell.to_string() == "Default")
            .ok_or(VariantError::DefaultColumnNotFound)?;

        let mut names: Vec<String> = Vec::with_capacity(data_rows);
        names.extend(rows.iter().skip(1).map(|row| row[name_index].to_string()));

        let mut defaults: Vec<Data> = Vec::with_capacity(data_rows);
        defaults.extend(rows.iter().skip(1).map(|row| row[default_index].clone()));

        let mut debugs: Option<Vec<Data>> = None;
        if debug {
            let debug_index = headers
                .iter()
                .position(|cell| cell.to_string() == "Debug")
                .ok_or(VariantError::OptionalColumnNotFound)?;

            let mut debug_vec: Vec<Data> = Vec::with_capacity(data_rows);
            debug_vec.extend(rows.iter().skip(1).map(|row| row[debug_index].clone()));

            debugs = Some(debug_vec);
        }

        let mut variants: Option<Vec<Data>> = None;
        if let Some(ref name) = variant {
            let variant_index = headers
                .iter()
                .position(|cell| cell.to_string() == *name)
                .ok_or(VariantError::OptionalColumnNotFound)?;

            let mut variant_vec: Vec<Data> = Vec::with_capacity(data_rows);
            variant_vec.extend(rows.iter().skip(1).map(|row| row[variant_index].clone()));

            variants = Some(variant_vec);
        };

        let mut sheets: HashMap<String, Range<Data>> = HashMap::new();
        for (name, sheet) in workbook.worksheets() {
            if name != "Main" {
                sheets.insert(name.to_string(), sheet);
            }
        }

        Ok(Self {
            names,
            defaults,
            debugs,
            variants,
            sheets,
        })
    }

    fn retrieve_cell_data(&self, name: &str) -> Result<Data, VariantError> {
        let index = self
            .names
            .iter()
            .position(|n| n == name)
            .ok_or(VariantError::RowNotFound)?;

        if let Some(debugs) = &self.debugs {
            if let Some(debug) = debugs.get(index) {
                if !matches!(debug, Data::Empty) {
                    return Ok(debug.clone());
                }
            }
        }

        if let Some(variants) = &self.variants {
            if let Some(variant) = variants.get(index) {
                if !matches!(variant, Data::Empty) {
                    return Ok(variant.clone());
                }
            }
        }

        if let Some(default) = self.defaults.get(index) {
            if !matches!(default, Data::Empty) {
                return Ok(default.clone());
            }
        }

        Err(VariantError::RowNotFound)
    }
}
