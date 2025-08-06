use crate::layout::LayoutError;

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
            _ => Err(LayoutError::MiscError("Invalid unit size".to_string())),
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
