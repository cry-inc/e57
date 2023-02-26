use crate::byte_stream::ByteStream;
use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::RecordType;
use crate::Result;

pub struct BitPack {}

impl BitPack {
    pub fn unpack_double(stream: &mut ByteStream, rt: &RecordType) -> Result<Vec<f64>> {
        match rt {
            RecordType::Double { .. } => {
                if stream.available() % 64 != 0 {
                    Error::invalid("Avalilabe bits do not match expected type size")?
                }
                let count = (stream.available() / 64) as usize;
                let mut result = Vec::with_capacity(count);
                for _ in 0..count {
                    let e = stream
                        .extract(64)
                        .internal_err("Unexpected error when extracing double from byte stream")?;
                    if e.bits != 64 || e.offset != 0 || e.data.len() != 8 {
                        Error::internal("Unexpected data after extracting double from byte stream")?
                    }
                    let s = e.data.as_slice();
                    let v = f64::from_le_bytes(s.try_into().internal_err(WRONG_OFFSET)?);
                    result.push(v);
                }
                Ok(result)
            }
            RecordType::Single { .. } => {
                if stream.available() % 32 != 0 {
                    Error::invalid("Avalilabe bits do not match expected type size")?
                }
                let count = (stream.available() / 32) as usize;
                let mut result = Vec::with_capacity(count);
                for _ in 0..count {
                    let e = stream
                        .extract(32)
                        .internal_err("Unexpected error when extracing float from byte stream")?;
                    if e.bits != 32 || e.offset != 0 || e.data.len() != 4 {
                        Error::internal("Unexpected data after extracting float from byte stream")?
                    }
                    let s = e.data.as_slice();
                    let v = f32::from_le_bytes(s.try_into().internal_err(WRONG_OFFSET)?);
                    result.push(v as f64);
                }
                Ok(result)
            }
            RecordType::ScaledInteger { min, max, scale } => {
                let bit_size = f64::ceil(f64::log2((*max as f64) - (*min as f64) + 1.0)) as u64;
                if bit_size % 8 != 0 {
                    Error::not_implemented(
                        "Scaled integers are only supported for multiples of 8 bit",
                    )?
                }
                let byte_size = f64::ceil((bit_size as f64) / 8.) as usize;
                let mut result = Vec::with_capacity((stream.available() / bit_size) as usize);
                loop {
                    if stream.available() < bit_size {
                        break;
                    }
                    let e = stream.extract(bit_size).internal_err(
                        "Unexpected error when extracing scaled integer from byte stream",
                    )?;
                    if e.bits != bit_size || e.offset != 0 {
                        Error::internal(
                            "Unexpected data after extracting scaled integer from byte stream",
                        )?
                    }
                    let mut tmp = [0_u8; 8];
                    tmp[..byte_size].copy_from_slice(&e.data);
                    let int_value = min + u64::from_le_bytes(tmp) as i64;
                    let float_value = int_value as f64 * scale;
                    result.push(float_value);
                }
                Ok(result)
            }
            RecordType::Integer { .. } => {
                Error::not_implemented(format!("Unpacking of {rt:?} as double is not supported"))
            }
        }
    }

    pub fn unpack_unit_float(stream: &mut ByteStream, rt: &RecordType) -> Result<Vec<f32>> {
        match rt {
            RecordType::Integer { min, max } => {
                let range = max - min;
                let bit_size = f64::ceil(f64::log2(range as f64 + 1.0)) as u64;
                if bit_size % 8 != 0 {
                    Error::not_implemented("Integers are only supported for multiples of 8 bit")?
                }
                let byte_size = f64::ceil((bit_size as f64) / 8.0) as usize;
                let mut result = Vec::with_capacity((stream.available() / bit_size) as usize);
                loop {
                    if stream.available() < bit_size {
                        break;
                    }
                    let e = stream.extract(bit_size).internal_err(
                        "Unexpected error when extracing scaled integer from byte stream",
                    )?;
                    if e.bits != bit_size || e.offset != 0 {
                        Error::internal(
                            "Unexpected data after extracting scaled integer from byte stream",
                        )?
                    }
                    let mut tmp = [0_u8; 8];
                    tmp[..byte_size].copy_from_slice(&e.data);
                    let int_value = u64::from_le_bytes(tmp) as i64;
                    let float_value = int_value as f32 / range as f32;
                    result.push(float_value);
                }
                Ok(result)
            }
            _ => Error::not_implemented(format!(
                "Unpacking of {rt:?} as unit float is not supported"
            )),
        }
    }

    pub fn unpack_u8(stream: &mut ByteStream, rt: &RecordType) -> Result<Vec<u8>> {
        match rt {
            RecordType::Integer { min, max } => {
                let range = max - min;
                let bit_size = f64::ceil(f64::log2(range as f64 + 1.0)) as u64;
                if bit_size != 1 {
                    Error::not_implemented(
                        "Unpacking to u8 is currently only possible for binary values",
                    )?
                }
                let mut result = Vec::with_capacity((stream.available() / bit_size) as usize);
                loop {
                    if stream.available() < bit_size {
                        break;
                    }
                    let e = stream
                        .extract(bit_size)
                        .internal_err("Unexpected error when extracing integer from byte stream")?;
                    if e.bits != bit_size {
                        Error::internal(
                            "Unexpected data after extracting integer from byte stream",
                        )?
                    }
                    let mask = 1 << e.offset;
                    result.push(if e.data[0] & mask == 0 { 0 } else { 1 });
                }
                Ok(result)
            }
            _ => Error::not_implemented(format!("Unpacking of {rt:?} as u8 is not supported")),
        }
    }
}
