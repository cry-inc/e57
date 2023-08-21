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
    let bits = T::bits();
    let av_bits = stream.available();
    if av_bits % bits != 0 {
        Error::invalid(format!(
            "Available bits {av_bits} do not match expected type size of {bits} bits"
        ))?
    }
    let count = av_bits / bits;
    for _ in 0..count {
        let e = stream.extract(bits).internal_err(format!(
            "Unexpected error when extracing {} from byte stream",
            std::any::type_name::<T>()
        ))?;
        let slice = &e.data[..e.data_len as usize];
        output(T::from_le_bytes(slice)?);
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
    if bit_size > 56 && bit_size != 64 {
        // These values can require 9 bytes before alignment
        // which would not fit into the u64 used for decoding!
        Error::not_implemented(format!("Integers with {bit_size} bits are not supported"))?
    }
    let mut mask = 0_u64;
    for i in 0..bit_size {
        mask |= 1 << i;
    }
    loop {
        if stream.available() < bit_size {
            break;
        }
        let e = stream
            .extract(bit_size)
            .internal_err("Unexpected error when extracing integer from byte stream")?;
        let uint_value = (u64::from_le_bytes(e.data) >> e.offset) & mask;
        let int_value = uint_value as i64 + min;
        output(int_value);
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
