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
        stream.extract_all(64, |data| {
            output.push_back(RecordValue::Double(f64::from_bits(data)));
        });
        Ok(())
    }

    pub fn unpack_singles(
        stream: &mut ByteStreamReadBuffer,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        stream.extract_all(32, |data| {
            output.push_back(RecordValue::Single(f32::from_bits(data as u32)));
        });
        Ok(())
    }

    pub fn unpack_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        let range = max as i128 - min as i128;
        let bit_count = range.ilog2() as usize + 1;
        stream.extract_all(bit_count, |data| {
            let int = (data as i64).wrapping_add(min);
            output.push_back(RecordValue::Integer(int));
        });
        Ok(())
    }

    pub fn unpack_scaled_ints(
        stream: &mut ByteStreamReadBuffer,
        min: i64,
        max: i64,
        output: &mut VecDeque<RecordValue>,
    ) -> Result<()> {
        let range = max as i128 - min as i128;
        let bit_count = range.ilog2() as usize + 1;
        stream.extract_all(bit_count, |data| {
            let int = (data as i64).wrapping_add(min);
            output.push_back(RecordValue::ScaledInteger(int));
        });
        Ok(())
    }
}
