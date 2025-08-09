use crate::error::*;

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

    // pass ref to vec to avoid copying
    pub fn to_bytes(&self, endianness: &Endianness) -> Vec<u8> {
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
}

pub enum EntrySource<'a, V: ConfigValue> {
    Value(&'a V),
    Name(String),
}

pub enum EntryType<'a, V: ConfigValue> {
    SingleEntry {
        type_str: String,
        source: EntrySource<'a, V>,
    },
    ArrayEntry {
        type_str: String,
        source: EntrySource<'a, V>,
        size: (i64, i64),
    },
    StringEntry {
        type_str: String,
        source: EntrySource<'a, V>,
        length: i64,
    },
    NestedTable(&'a dyn ConfigTable<Value = V>),
}

pub trait ConfigValue {
    fn as_integer(&self) -> Option<i64>;
    fn as_float(&self) -> Option<f64>;
    fn as_string(&self) -> Option<&str>;
    fn as_size_tuple(&self) -> Result<(i64, i64), NvmError>;
    fn as_bool(&self) -> Option<bool>;
    fn as_table(&self) -> Option<&dyn ConfigTable<Value = Self>>;

    fn classify_entry(&self) -> Result<EntryType<Self>, NvmError>
    where
        Self: Sized,
    {
        // This should only ever be called on a table
        let table = self.as_table().ok_or(NvmError::RecursionFailed(
            "couldn't retrieve table where one was expected.".to_string(),
        ))?;

        // If type exists we assume it's a data entry to extract
        if let Some(value) = table.get("type") {
            let type_str = value
                .as_string()
                .ok_or(NvmError::FailedToExtract(
                    "Non-string type found.".to_string(),
                ))?
                .to_string();

            // Grab the size option if it exists
            let size = match table.get("size") {
                Some(size) => Some(size.as_size_tuple()?),
                None => None,
            };

            let source = match (table.get("value"), table.get("name")) {
                (Some(value), None) => EntrySource::Value(value),
                (None, Some(name)) => EntrySource::Name(
                    name.as_string()
                        .ok_or(NvmError::FailedToExtract(
                            "Non-string name found.".to_string(),
                        ))?
                        .to_string(),
                ),
                _ => {
                    return Err(NvmError::RecursionFailed(
                        "Found neither/both value and name in the same entry.".to_string(),
                    ));
                }
            };

            match size {
                Some((rows, 0)) => Ok(EntryType::StringEntry {
                    type_str,
                    source,
                    length: rows,
                }),
                Some((rows, cols)) => Ok(EntryType::ArrayEntry {
                    type_str,
                    source,
                    size: (rows, cols),
                }),
                None => Ok(EntryType::SingleEntry { type_str, source }),
            }

        // No type means we continue to the next level of recursion
        } else {
            Ok(EntryType::NestedTable(table))
        }
    }

    fn export_datavalue(&self, type_str: &str) -> Result<DataValue, NvmError> {
        let value = match type_str {
            "u8" => DataValue::U8(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as u8,
            ),
            "u16" => DataValue::U16(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as u16,
            ),
            "u32" => DataValue::U32(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as u32,
            ),
            "u64" => DataValue::U64(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as u64,
            ),
            "i8" => DataValue::I8(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as i8,
            ),
            "i16" => DataValue::I16(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as i16,
            ),
            "i32" => DataValue::I32(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as i32,
            ),
            "i64" => DataValue::I64(
                self.as_integer()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as i64,
            ),
            "f32" => DataValue::F32(
                self.as_float()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as f32,
            ),
            "f64" => DataValue::F64(
                self.as_float()
                    .ok_or(NvmError::DataValueExportFailed(type_str.to_string()))?
                    as f64,
            ),
            _ => {
                return Err(NvmError::DataValueExportFailed(
                    "unsupported data type: ".to_string() + type_str,
                ));
            }
        };
        Ok(value)
    }
}

impl ConfigValue for toml::Value {
    fn as_integer(&self) -> Option<i64> {
        match self {
            toml::Value::Integer(n) => Some(*n),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match self {
            toml::Value::Float(f) => Some(*f),
            toml::Value::Integer(n) => Some(*n as f64),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            toml::Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_size_tuple(&self) -> Result<(i64, i64), NvmError> {
        match self {
            // Matching specifically against the expected 1 or 2 elements of the size list
            toml::Value::Array(array) if (1..=2).contains(&array.len()) => {
                let rows = array[0].as_integer().ok_or(NvmError::FailedToExtract(
                    "Non-integer number of rows found.".to_string(),
                ))?;

                let cols = if let Some(v) = array.get(1) {
                    v.as_integer().ok_or(NvmError::FailedToExtract(
                        "Non-integer number of columns found.".to_string(),
                    ))?
                } else {
                    0
                };

                Ok((rows, cols))
            }
            _ => Err(NvmError::FailedToExtract(
                "Invalid size array found.".to_string(),
            )),
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            toml::Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    fn as_table(&self) -> Option<&dyn ConfigTable<Value = Self>> {
        match self {
            toml::Value::Table(table) => Some(table),
            _ => None,
        }
    }
}
pub trait ConfigTable {
    type Value: ConfigValue;

    fn get(&self, key: &str) -> Option<&Self::Value>;
    fn iter(&self) -> Box<dyn Iterator<Item = (&str, &Self::Value)> + '_>;
    fn remove(&mut self, key: &str) -> Option<Self::Value>;

    fn from_value(value: Self::Value) -> Option<Self>
    where
        Self: Sized;
}

impl ConfigTable for toml::Table {
    type Value = toml::Value;

    fn get(&self, key: &str) -> Option<&Self::Value> {
        self.get(key)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&str, &Self::Value)> + '_> {
        Box::new(self.iter().map(|(k, v)| (k.as_str(), v)))
    }

    fn remove(&mut self, key: &str) -> Option<Self::Value> {
        self.remove(key)
    }

    fn from_value(value: Self::Value) -> Option<Self> {
        match value {
            toml::Value::Table(table) => Some(table),
            _ => None,
        }
    }
}
