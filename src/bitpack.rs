use crate::bs_read::ByteStreamReadBuffer;
use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::RecordValue;
use crate::Result;

pub struct BitPack;

#[inline]
fn unpack_fp<T: FromBytes>(stream: &mut ByteStreamReadBuffer) -> Result<Vec<T>> {
    let bits = T::bits();
    let av_bits = stream.available();
    if av_bits % bits != 0 {
        Error::invalid(format!(
            "Available bits {av_bits} do not match expected type size of {bits} bits"
        ))?
    }
    let count = av_bits / bits;
    let mut result = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let e = stream.extract(bits).internal_err(format!(
            "Unexpected error when extracing {} from byte stream",
            std::any::type_name::<T>()
        ))?;
        result.push(T::from_le_bytes(e.data.as_slice())?);
    }
    Ok(result)
}

#[inline]
fn unpack_int(stream: &mut ByteStreamReadBuffer, min: i64, max: i64) -> Result<Vec<i64>> {
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
    let mut result = Vec::with_capacity((stream.available() / bit_size) as usize);
    loop {
        if stream.available() < bit_size {
            break;
        }
        let e = stream
            .extract(bit_size)
            .internal_err("Unexpected error when extracing integer from byte stream")?;
        let mut tmp = [0_u8; 8];
        tmp[..e.data.len()].copy_from_slice(&e.data);
        let uint_value = (u64::from_le_bytes(tmp) >> e.offset) & mask;
        let int_value = uint_value as i64 + min;
        result.push(int_value);
    }
    Ok(result)
}

impl BitPack {
    pub fn unpack_doubles(stream: &mut ByteStreamReadBuffer) -> Result<Vec<RecordValue>> {
        let doubles = unpack_fp::<f64>(stream)?;
        Ok(doubles.iter().map(|d| RecordValue::Double(*d)).collect())
    }

    pub fn unpack_singles(stream: &mut ByteStreamReadBuffer) -> Result<Vec<RecordValue>> {
        let singles = unpack_fp::<f32>(stream)?;
        Ok(singles.iter().map(|s| RecordValue::Single(*s)).collect())
    }

    pub fn unpack_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
    ) -> Result<Vec<RecordValue>> {
        let ints = unpack_int(stream, min, max)?;
        Ok(ints.iter().map(|i| RecordValue::Integer(*i)).collect())
    }

    pub fn unpack_scaled_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
    ) -> Result<Vec<RecordValue>> {
        let ints = unpack_int(stream, min, max)?;
        Ok(ints
            .iter()
            .map(|i| RecordValue::ScaledInteger(*i))
            .collect())
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
