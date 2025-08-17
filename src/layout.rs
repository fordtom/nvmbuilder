use crate::error::*;
use crate::types::*;
use crate::variants::DataSheet;

// impl Config {
//     pub fn build_bytestream(&self, data_sheet: &DataSheet) -> Result<Vec<u8>, NvmError> {
//         let mut buffer = Vec::with_capacity(self.block.header.length as usize);
//         let mut offset = 0;

//         // set padding to block.header.padding if present, else fallback to settings.padding
//         let padding = self.block.header.padding.unwrap_or(self.settings.padding);
//         let padding = DataValue::U8(padding);

//         Self::build_bytestream_inner(
//             &self.block.data,
//             data_sheet,
//             &mut buffer,
//             &mut offset,
//             &self.settings.endianness,
//             &padding,
//         )?;
//         Ok(buffer)
//     }

//     fn build_bytestream_inner(
//         table: &Entry,
//         data_sheet: &DataSheet,
//         buffer: &mut Vec<u8>,
//         offset: &mut usize,
//         endianness: &Endianness,
//         padding: &DataValue,
//     ) -> Result<(), NvmError> {
//         for (_, v) in table.iter() {
//             match v.classify_entry() {
//                 // Handle single value
//                 Ok(EntryType::SingleEntry { type_str, source }) => {
//                     let data_value = match source {
//                         EntrySource::Value(value) => value.export_datavalue(&type_str)?,
//                         EntrySource::Name(name) => {
//                             data_sheet.retrieve_cell_data(&name, &type_str)?
//                         }
//                     };
//                     buffer.extend(data_value.to_bytes(endianness));
//                     *offset += data_value.size_bytes();
//                 }

//                 // Handle string (TODO)

//                 // Handle array (TODO)

//                 // If we have a nested table we recurse
//                 Ok(EntryType::NestedTable(nested_table)) => {
//                     Self::build_bytestream_inner(
//                         nested_table,
//                         data_sheet,
//                         buffer,
//                         offset,
//                         endianness,
//                         padding,
//                     )?;
//                 }

//                 // Pass up errors
//                 Err(e) => {
//                     return Err(e);
//                 }

//                 // temporary before we implement strings/arrays
//                 _ => {
//                     return Err(NvmError::FailedToExtract(
//                         "unsupported entry type.".to_string(),
//                     ));
//                 }
//             }
//         }
//         Ok(())
//     }
// }
