use crate::error::*;
use crate::schema::*;
use crate::variants::DataSheet;

impl Block {
    pub fn build_bytestream(
        &self,
        data_sheet: &DataSheet,
        settings: &Settings,
    ) -> Result<Vec<u8>, NvmError> {
        let mut buffer = Vec::with_capacity(self.header.length as usize);
        let mut offset = 0;

        Self::build_bytestream_inner(
            &self.data,
            data_sheet,
            &mut buffer,
            &mut offset,
            &settings.endianness,
            &self.header.padding,
        )?;
        Ok(buffer)
    }

    fn build_bytestream_inner(
        table: &Entry,
        data_sheet: &DataSheet,
        buffer: &mut Vec<u8>,
        offset: &mut usize,
        endianness: &Endianness,
        padding: &u8,
    ) -> Result<(), NvmError> {
        match table {
            Entry::Leaf(leaf) => {
                let bytes = leaf.emit_bytes(data_sheet, endianness, padding)?;
                *offset += bytes.len();
                buffer.extend(bytes);
            }
            Entry::Branch(branch) => {
                for (_, v) in branch.iter() {
                    Self::build_bytestream_inner(
                        v, data_sheet, buffer, offset, endianness, padding,
                    )?;
                }
            }
        }
        Ok(())
    }
}
