// use crate::layout::{
//     LayoutError,
//     types::{DataValue, TypeSpec},
// };

// pub fn extract_datavalue(value: &toml::Value) -> Result<Vec<DataValue>, LayoutError> {
//     match value {
//         toml::Value::Table(table) => {
//             let type_value = table
//                 .get("type")
//                 .ok_or(LayoutError::BadDataValueExtraction)?;
//             let typespec = TypeSpec::from_value(type_value)?;
//             typespec.extract_datavalues(value)
//         }
//         _ => Err(LayoutError::BadDataValueExtraction),
//     }
// }
