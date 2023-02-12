use crate::error::Converter;
use crate::Error;
use crate::RecordType;
use crate::Result;

pub struct BitPack {}

impl BitPack {
    pub fn unpack_double(buffer: &[u8], rt: &RecordType) -> Result<Vec<f64>> {
        match rt {
            RecordType::Double { .. } => {
                if buffer.len() % 8 != 0 {
                    Error::invalid("Buffer size does not match record type size")?
                }
                let count = buffer.len() / 8;
                let mut result = Vec::with_capacity(count);
                for i in 0..count {
                    let s = i * 8;
                    let e = (i + 1) * 8;
                    let v = f64::from_le_bytes(
                        buffer[s..e]
                            .try_into()
                            .internal_err("Unexpected offset issue")?,
                    );
                    result.push(v);
                }
                Ok(result)
            }
            RecordType::Single { .. } => {
                if buffer.len() % 4 != 0 {
                    Error::invalid("Buffer size does not match record type size")?
                }
                let count = buffer.len() / 4;
                let mut result = Vec::with_capacity(count);
                for i in 0..count {
                    let s = i * 4;
                    let e = (i + 1) * 4;
                    let v = f32::from_le_bytes(
                        buffer[s..e]
                            .try_into()
                            .internal_err("Unexpected offset issue")?,
                    );
                    result.push(v as f64);
                }
                Ok(result)
            }
            RecordType::Integer { .. } => todo!(),
        }
    }
}
