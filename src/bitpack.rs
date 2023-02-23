use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::RecordType;
use crate::Result;

pub struct BitPack {}

impl BitPack {
    pub fn unpack_double(buffer: &[u8], rt: &RecordType) -> Result<Vec<f64>> {
        match rt {
            RecordType::Double { .. } => {
                if buffer.len() % 8 != 0 {
                    Error::invalid("Buffer size does not match expected type size")?
                }
                let count = buffer.len() / 8;
                let mut result = Vec::with_capacity(count);
                for i in 0..count {
                    let s = i * 8;
                    let e = (i + 1) * 8;
                    let v = f64::from_le_bytes(buffer[s..e].try_into().internal_err(WRONG_OFFSET)?);
                    result.push(v);
                }
                Ok(result)
            }
            RecordType::Single { .. } => {
                if buffer.len() % 4 != 0 {
                    Error::invalid("Buffer size does not match expected type size")?
                }
                let count = buffer.len() / 4;
                let mut result = Vec::with_capacity(count);
                for i in 0..count {
                    let s = i * 4;
                    let e = (i + 1) * 4;
                    let v = f32::from_le_bytes(buffer[s..e].try_into().internal_err(WRONG_OFFSET)?);
                    result.push(v as f64);
                }
                Ok(result)
            }
            RecordType::ScaledInteger { min, max, scale } => {
                let bit_size = f64::ceil(f64::log2((*max as f64) - (*min as f64) + 1.0)) as usize;
                if bit_size % 8 != 0 {
                    Error::not_implemented(
                        "Scaled integers are only supported for multiples of 8 bit",
                    )?
                }
                let byte_size = f64::ceil((bit_size as f64) / 8.0) as usize;
                let mut result = Vec::new();
                let mut count = 0;
                loop {
                    let byte_index = (count * bit_size) / 8;
                    if byte_index + byte_size > buffer.len() {
                        break;
                    }
                    let mut tmp = [0_u8; 8];
                    tmp[..byte_size].copy_from_slice(&buffer[byte_index..(byte_index + byte_size)]);
                    let int_value = min + u64::from_le_bytes(tmp) as i64;
                    let float_value = int_value as f64 * scale;
                    result.push(float_value);
                    count += 1;
                }
                Ok(result)
            }
            RecordType::Integer { .. } => {
                Error::not_implemented(format!("Unpacking of {rt:?} as double is not supported"))
            }
        }
    }

    pub fn unpack_float(buffer: &[u8], rt: &RecordType) -> Result<Vec<f32>> {
        match rt {
            RecordType::Double { .. } => {
                Error::not_implemented(format!("Unpacking of {rt:?} as float is not supported"))
            }
            RecordType::Single { .. } => {
                Error::not_implemented(format!("Unpacking of {rt:?} as float is not supported"))
            }
            RecordType::ScaledInteger { .. } => {
                Error::not_implemented(format!("Unpacking of {rt:?} as float is not supported"))
            }
            RecordType::Integer { min, max } => {
                let range = max - min;
                let bit_size = f64::ceil(f64::log2(range as f64 + 1.0)) as usize;
                if bit_size % 8 != 0 {
                    Error::not_implemented("Integers are only supported for multiples of 8 bit")?
                }
                let byte_size = f64::ceil((bit_size as f64) / 8.0) as usize;
                let mut result = Vec::new();
                let mut count = 0;
                loop {
                    let byte_index = (count * bit_size) / 8;
                    if byte_index + byte_size > buffer.len() {
                        break;
                    }
                    let mut tmp = [0_u8; 8];
                    tmp[..byte_size].copy_from_slice(&buffer[byte_index..(byte_index + byte_size)]);
                    let int_value = u64::from_le_bytes(tmp) as i64;
                    let float_value = int_value as f32 / range as f32;
                    result.push(float_value);
                    count += 1;
                }
                Ok(result)
            }
        }
    }
}
