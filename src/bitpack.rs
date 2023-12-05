use crate::bs_read::ByteStreamReadBuffer;
use crate::RecordValue;
use crate::Result;
use std::collections::VecDeque;

pub struct BitPack;

impl BitPack {
    pub fn unpack_doubles(
        stream: &mut ByteStreamReadBuffer,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        while let Some(data) = stream.extract(64) {
            let bytes = data.to_le_bytes();
            let value = f64::from_le_bytes(bytes);
            output.push_back(RecordValue::Double(value));
        }
        Ok(())
    }

    pub fn unpack_singles(
        stream: &mut ByteStreamReadBuffer,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        while let Some(data) = stream.extract(32) {
            let bytes = (data as u32).to_le_bytes();
            let value = f32::from_le_bytes(bytes);
            output.push_back(RecordValue::Single(value));
        }
        Ok(())
    }

    pub fn unpack_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        let range = max as i128 - min as i128;
        let bits = range.ilog2() as usize + 1;
        let mask = ((1_u128 << bits) - 1) as u64;
        while let Some(uint) = stream.extract(bits) {
            let int = (uint & mask) as i128 + min as i128;
            output.push_back(RecordValue::Integer(int as i64));
        }
        Ok(())
    }

    pub fn unpack_scaled_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        let range = max as i128 - min as i128;
        let bits = range.ilog2() as usize + 1;
        let mask = ((1_u128 << bits) - 1) as u64;
        while let Some(uint) = stream.extract(bits) {
            let int = (uint & mask) as i128 + min as i128;
            output.push_back(RecordValue::ScaledInteger(int as i64));
        }
        Ok(())
    }
}
