#![allow(dead_code, unused_variables)]

use crate::layout::LayoutError;

#[derive(Debug, Clone)]
pub enum CrcLocation {
    Start,
    End,
    Address(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug, Clone)]
pub enum DataValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl DataValue {
    pub fn size_bytes(&self) -> usize {
        match self {
            DataValue::U8(_) | DataValue::I8(_) => 1,
            DataValue::U16(_) | DataValue::I16(_) => 2,
            DataValue::U32(_) | DataValue::I32(_) | DataValue::F32(_) => 4,
            DataValue::U64(_) | DataValue::I64(_) | DataValue::F64(_) => 8,
        }
    }

    pub fn to_bytes(&self, endianness: Endianness) -> Vec<u8> {
        match (self, endianness) {
            // Single byte values - endianness doesn't matter
            (DataValue::U8(val), _) => vec![*val],
            (DataValue::I8(val), _) => vec![*val as u8],

            // Multi-byte unsigned integers
            (DataValue::U16(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::U16(val), Endianness::Big) => val.to_be_bytes().to_vec(),
            (DataValue::U32(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::U32(val), Endianness::Big) => val.to_be_bytes().to_vec(),
            (DataValue::U64(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::U64(val), Endianness::Big) => val.to_be_bytes().to_vec(),

            // Multi-byte signed integers
            (DataValue::I16(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::I16(val), Endianness::Big) => val.to_be_bytes().to_vec(),
            (DataValue::I32(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::I32(val), Endianness::Big) => val.to_be_bytes().to_vec(),
            (DataValue::I64(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::I64(val), Endianness::Big) => val.to_be_bytes().to_vec(),

            // Floating point numbers
            (DataValue::F32(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::F32(val), Endianness::Big) => val.to_be_bytes().to_vec(),
            (DataValue::F64(val), Endianness::Little) => val.to_le_bytes().to_vec(),
            (DataValue::F64(val), Endianness::Big) => val.to_be_bytes().to_vec(),
        }
    }

    pub fn from_toml_cell(type_str: &str, value: &toml::Value) -> Result<Self, LayoutError> {
        match type_str {
            "u8" => Ok(DataValue::U8(Self::parse_uint(value)? as u8)),
            "u16" => Ok(DataValue::U16(Self::parse_uint(value)? as u16)),
            "u32" => Ok(DataValue::U32(Self::parse_uint(value)? as u32)),
            "u64" => Ok(DataValue::U64(Self::parse_uint(value)? as u64)),
            "i8" => Ok(DataValue::I8(Self::parse_int(value)? as i8)),
            "i16" => Ok(DataValue::I16(Self::parse_int(value)? as i16)),
            "i32" => Ok(DataValue::I32(Self::parse_int(value)? as i32)),
            "i64" => Ok(DataValue::I64(Self::parse_int(value)? as i64)),
            "f32" => Ok(DataValue::F32(Self::parse_float(value)? as f32)),
            "f64" => Ok(DataValue::F64(Self::parse_float(value)? as f64)),
            _ => Err(LayoutError::InvalidCell),
        }
    }

    fn parse_uint(value: &toml::Value) -> Result<u64, LayoutError> {
        match value {
            toml::Value::Integer(n) if *n >= 0 => {
                u64::try_from(*n).map_err(|_| LayoutError::InvalidCell)
            }
            _ => Err(LayoutError::InvalidCell),
        }
    }

    fn parse_int(value: &toml::Value) -> Result<i64, LayoutError> {
        match value {
            toml::Value::Integer(n) => Ok(*n),
            _ => Err(LayoutError::InvalidCell),
        }
    }

    fn parse_float(value: &toml::Value) -> Result<f64, LayoutError> {
        match value {
            toml::Value::Float(n) => Ok(*n),
            toml::Value::Integer(n) => Ok(*n as f64),
            _ => Err(LayoutError::InvalidCell),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AddressWidth {
    Bits16,
    Bits32,
    Bits64,
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryUnitSize {
    Bytes1 = 1,
    Bytes2 = 2,
    Bytes4 = 4,
    Bytes8 = 8,
}

impl MemoryUnitSize {
    pub fn from_bytes(bytes: usize) -> Result<Self, LayoutError> {
        match bytes {
            1 => Ok(MemoryUnitSize::Bytes1),
            2 => Ok(MemoryUnitSize::Bytes2),
            4 => Ok(MemoryUnitSize::Bytes4),
            8 => Ok(MemoryUnitSize::Bytes8),
            _ => Err(LayoutError::InvalidUnitSize), // Add this error variant
        }
    }

    pub fn as_bytes(self) -> usize {
        self as usize
    }

    pub fn align_size(self, size: usize) -> usize {
        let unit = self.as_bytes();
        (size + unit - 1) / unit * unit
    }
}

#[derive(Debug, Clone)]
pub enum TypeSpec {
    Simple(String),
    Array {
        count: usize,
        element_types: Vec<String>,
    },
}

impl TypeSpec {
    pub fn from_value(type_string: &toml::Value) -> Result<Self, LayoutError> {
        match type_string {
            toml::Value::String(s) => Ok(TypeSpec::Simple(s.clone())),
            toml::Value::Array(_) => {
                // Will handle later
                Err(LayoutError::InvalidCell)
            }
            _ => Err(LayoutError::InvalidCell),
        }
    }

    pub fn extract_datavalues(&self, entry: &toml::Value) -> Result<Vec<DataValue>, LayoutError> {
        match self {
            TypeSpec::Simple(type_string) => {
                let value = entry.get("value").ok_or(LayoutError::InvalidCell)?;
                let data_value = DataValue::from_toml_cell(&type_string, value)?;
                Ok(vec![data_value])
            }
            TypeSpec::Array {
                count,
                element_types,
            } => {
                // TODO
                Err(LayoutError::InvalidCell)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Value;

    #[test]
    fn test_memory_unit_size_from_bytes() {
        assert!(matches!(
            MemoryUnitSize::from_bytes(1),
            Ok(MemoryUnitSize::Bytes1)
        ));
        assert!(matches!(
            MemoryUnitSize::from_bytes(2),
            Ok(MemoryUnitSize::Bytes2)
        ));
        assert!(matches!(
            MemoryUnitSize::from_bytes(4),
            Ok(MemoryUnitSize::Bytes4)
        ));
        assert!(matches!(
            MemoryUnitSize::from_bytes(8),
            Ok(MemoryUnitSize::Bytes8)
        ));
        assert!(matches!(
            MemoryUnitSize::from_bytes(3),
            Err(LayoutError::InvalidUnitSize)
        ));
    }

    #[test]
    fn test_memory_unit_size_align() {
        let unit2 = MemoryUnitSize::Bytes2;
        assert_eq!(unit2.align_size(1), 2); // 1 -> 2
        assert_eq!(unit2.align_size(2), 2); // 2 -> 2  
        assert_eq!(unit2.align_size(3), 4); // 3 -> 4
        assert_eq!(unit2.align_size(4), 4); // 4 -> 4
    }

    #[test]
    fn test_datavalue_from_toml_cell() {
        // Test u32
        let value = Value::Integer(0x1234);
        let result = DataValue::from_toml_cell("u32", &value).unwrap();
        assert!(matches!(result, DataValue::U32(0x1234)));

        // Test u8
        let value = Value::Integer(255);
        let result = DataValue::from_toml_cell("u8", &value).unwrap();
        assert!(matches!(result, DataValue::U8(255)));

        // Test invalid type
        let value = Value::Integer(42);
        let result = DataValue::from_toml_cell("invalid", &value);
        assert!(matches!(result, Err(LayoutError::InvalidCell)));
    }

    #[test]
    fn test_datavalue_to_bytes() {
        // Test u32 little endian
        let val = DataValue::U32(0x12345678);
        let bytes = val.to_bytes(Endianness::Little);
        assert_eq!(bytes, vec![0x78, 0x56, 0x34, 0x12]);

        // Test u32 big endian
        let bytes = val.to_bytes(Endianness::Big);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);

        // Test u8 (endianness doesn't matter)
        let val = DataValue::U8(0xFF);
        let bytes = val.to_bytes(Endianness::Little);
        assert_eq!(bytes, vec![0xFF]);
    }

    #[test]
    fn test_typespec_simple() {
        let type_value = Value::String("u32".to_string());
        let typespec = TypeSpec::from_value(&type_value).unwrap();
        assert!(matches!(typespec, TypeSpec::Simple(s) if s == "u32"));
    }

    #[test]
    fn test_typespec_extract_simple_datavalue() {
        // Create a TOML table that looks like: { value = 0x1234, type = "u32" }
        let mut table = toml::Table::new();
        table.insert("value".to_string(), Value::Integer(0x1234));
        table.insert("type".to_string(), Value::String("u32".to_string()));
        let entry = Value::Table(table);

        let typespec = TypeSpec::Simple("u32".to_string());
        let result = typespec.extract_datavalues(&entry).unwrap();

        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], DataValue::U32(0x1234)));
    }

    #[test]
    fn test_typespec_array_errors() {
        // Arrays should error for now
        let type_value = Value::Array(vec![Value::Integer(10), Value::String("u16".to_string())]);
        let result = TypeSpec::from_value(&type_value);
        assert!(matches!(result, Err(LayoutError::InvalidCell)));
    }
}
