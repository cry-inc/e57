use crate::bs_read::ByteStreamReadBuffer;
use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::RecordValue;
use crate::Result;
use std::collections::VecDeque;

pub struct BitPack;

#[inline]
fn unpack_fp<T: FromBytes>(
    stream: &mut ByteStreamReadBuffer,
    output: &mut dyn FnMut(T),
) -> Result<()> {
    let av_bits = stream.available();
    let bits = T::bits();
    if av_bits % bits != 0 {
        Error::invalid(format!(
            "Available bits {av_bits} do not match expected type size of {bits} bits"
        ))?
    }
    loop {
        let extracted = stream.extract(bits);
        if let Some(data) = extracted {
            let bytes = (bits / 8) as usize;
            let slice = &data.to_le_bytes()[..bytes];
            output(T::from_le_bytes(slice)?);
        } else {
            break;
        }
    }
    Ok(())
}

#[inline]
fn unpack_int(
    stream: &mut ByteStreamReadBuffer,
    min: i64,
    max: i64,
    output: &mut dyn FnMut(i64),
) -> Result<()> {
    let range = max - min;
    let bit_size = f64::ceil(f64::log2(range as f64 + 1.0)) as u64;
    let mask = (1_u64 << bit_size) - 1;
    loop {
        let extracted = stream.extract(bit_size);
        if let Some(uint_value) = extracted {
            let int_value = (uint_value & mask) as i64 + min;
            output(int_value);
        } else {
            break;
        }
    }
    Ok(())
}

impl BitPack {
    pub fn unpack_doubles(
        stream: &mut ByteStreamReadBuffer,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        unpack_fp::<f64>(stream, &mut |v: f64| {
            output.push_back(RecordValue::Double(v))
        })
    }

    pub fn unpack_singles(
        stream: &mut ByteStreamReadBuffer,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        unpack_fp::<f32>(stream, &mut |v: f32| {
            output.push_back(RecordValue::Single(v))
        })
    }

    pub fn unpack_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        unpack_int(stream, min, max, &mut |v: i64| {
            output.push_back(RecordValue::Integer(v))
        })
    }

    pub fn unpack_scaled_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        unpack_int(stream, min, max, &mut |v: i64| {
            output.push_back(RecordValue::ScaledInteger(v))
        })
    }
}

trait FromBytes: Sized {
    fn from_le_bytes(bytes: &[u8]) -> Result<Self>;
    fn bits() -> u64 {
        std::mem::size_of::<Self>() as u64 * 8
    }
}

impl FromBytes for f64 {
    #[inline]
    fn from_le_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(f64::from_le_bytes(
            bytes.try_into().internal_err(WRONG_OFFSET)?,
        ))
    }
}

impl FromBytes for f32 {
    #[inline]
    fn from_le_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(f32::from_le_bytes(
            bytes.try_into().internal_err(WRONG_OFFSET)?,
        ))
    }
}
