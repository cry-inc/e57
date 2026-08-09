#[derive(Clone)]
pub struct ByteStreamReadBuffer {
    buffer: Vec<u8>,
    offset: usize,
}

impl ByteStreamReadBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
        }
    }

    /// Append bytes read from a reader to the end of the stream.
    /// Reads directly into the internal buffer to avoid
    /// an extra copy through an intermediate buffer.
    pub fn append_from_reader(
        &mut self,
        reader: &mut impl std::io::Read,
        bytes: usize,
    ) -> std::io::Result<()> {
        let consumed_bytes = self.offset / 8;
        if consumed_bytes > 0 {
            self.buffer.drain(..consumed_bytes);
            self.offset -= consumed_bytes * 8;
        }
        let old_len = self.buffer.len();
        self.buffer.resize(old_len + bytes, 0);
        reader.read_exact(&mut self.buffer[old_len..])
    }

    /// Extracts a single value with 64 bits or less from the byte stream and returns it as u64.
    /// The returned u64 might contain more than the requested number of bits.
    /// Make sure to ignore/mask the additional bits!
    /// Returns None if the request cannot be satisfied.
    fn extract_single(&mut self, bits: usize) -> Option<u64> {
        if self.available() < bits {
            return None;
        }

        let start_offset = self.offset / 8;
        let end_offset = (self.offset + bits).div_ceil(8); // Integer division with rounding up
        let offset = self.offset % 8;

        let mut data = [0; 16];
        let data_len = end_offset - start_offset;
        let dst = &mut data[..data_len];
        let src = &self.buffer[start_offset..end_offset];
        dst.copy_from_slice(src);

        self.offset += bits;
        let data = u128::from_le_bytes(data) >> offset;
        Some(data as u64)
    }

    /// Extract all complete values with the requested bit size
    /// and pass them to the supplied closure.
    /// Unlike `extract()` the values are already masked,
    /// meaning all additional upper bits are set to zero.
    /// The bit size must be between 1 and 64.
    pub fn extract_all(&mut self, bits: usize, mut out: impl FnMut(u64)) {
        if !(1..=64).contains(&bits) {
            return;
        }

        let count = self.available() / bits;
        if count == 0 {
            return;
        }

        if self.offset.is_multiple_of(8) && bits.is_multiple_of(8) {
            // Fast path for byte-aligned values without bit shifting
            let size = bits / 8;
            let start = self.offset / 8;
            let end = start + count * size;
            for chunk in self.buffer[start..end].chunks_exact(size) {
                let mut bytes = [0_u8; 8];
                bytes[..size].copy_from_slice(chunk);
                out(u64::from_le_bytes(bytes));
            }
            self.offset += count * bits;
        } else if bits <= 57 {
            // Fast path with bit accumulator that avoids
            // copying the data for each extracted value.
            // Only works for up to 57 bits because the accumulator
            // must always have enough space for one additional byte.
            let mask = (1_u64 << bits) - 1;
            let mut byte_pos = self.offset / 8;
            let bit_offset = self.offset % 8;
            let mut acc = 0_u64;
            let mut acc_bits = 0;
            if bit_offset != 0 {
                acc = (self.buffer[byte_pos] >> bit_offset) as u64;
                acc_bits = 8 - bit_offset;
                byte_pos += 1;
            }
            for _ in 0..count {
                while acc_bits < bits {
                    acc |= (self.buffer[byte_pos] as u64) << acc_bits;
                    byte_pos += 1;
                    acc_bits += 8;
                }
                out(acc & mask);
                acc >>= bits;
                acc_bits -= bits;
            }
            self.offset += count * bits;
        } else {
            // Generic slower fallback for unaligned values with more than 57 bits
            let mask = if bits == 64 {
                u64::MAX
            } else {
                (1_u64 << bits) - 1
            };
            while let Some(value) = self.extract_single(bits) {
                out(value & mask);
            }
        }
    }

    /// Returns the number of available bits in the stream
    pub fn available(&self) -> usize {
        (self.buffer.len() * 8) - self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to easily put data into the BS for tests
    fn append(bs: &mut ByteStreamReadBuffer, data: &[u8]) {
        let mut cursor = std::io::Cursor::new(data);
        bs.append_from_reader(&mut cursor, data.len()).unwrap();
    }

    #[test]
    fn empty() {
        let mut bs = ByteStreamReadBuffer::new();
        assert_eq!(bs.available(), 0);
        let result = bs.extract_single(0).unwrap();
        assert_eq!(result, 0);
        assert_eq!(bs.available(), 0);
        assert!(bs.extract_single(1).is_none());
    }

    #[test]
    fn append_and_extract_bits() {
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[255]);

        assert_eq!(bs.available(), 8);
        let result = bs.extract_single(2).unwrap();
        assert_eq!(result, 255);

        assert_eq!(bs.available(), 6);
        let result = bs.extract_single(6).unwrap();
        assert_eq!(result, 63);

        assert_eq!(bs.available(), 0);
        assert!(bs.extract_single(1).is_none());
    }

    #[test]
    fn append_and_extract_bytes() {
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[23, 42, 13]);
        bs.extract_single(2).unwrap();

        assert_eq!(bs.available(), 22);
        let result = bs.extract_single(22).unwrap();
        assert_eq!(result, 215685);
    }

    #[test]
    fn remove_consume_when_appending() {
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[1, 2, 3, 4, 5]);
        bs.extract_single(4 * 8 + 2).unwrap();

        // We append one byte and the buffer should become smaller
        // because all fully consumed bytes are removed.
        append(&mut bs, &[6]);
        assert!(bs.buffer.len() == 2);

        // Offsets are updated correctly appended
        // data can be extracted as expected.
        let result = bs.extract_single(14).unwrap();
        assert_eq!(result, 385);
    }

    #[test]
    fn extract_all_aligned() {
        // Byte-aligned fast path with 16 bit values
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[0x01, 0x00, 0xff, 0xff, 0x34, 0x12, 0x99]);
        let mut values = Vec::new();
        bs.extract_all(16, |v| values.push(v));
        assert_eq!(values, [0x0001, 0xffff, 0x1234]);
        assert_eq!(bs.available(), 8);
    }

    #[test]
    fn extract_all_accumulator() {
        // Bit accumulator fast path with values that are not byte-aligned
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[0b10101010, 0b11001100, 0b11110000]);
        let mut values = Vec::new();
        bs.extract_all(3, |v| values.push(v));
        assert_eq!(values.len(), 8);
        assert_eq!(values, [2, 5, 2, 6, 4, 1, 4, 7]);
        assert_eq!(bs.available(), 0);
    }

    #[test]
    fn extract_single_unaligned_8bit() {
        // Unaligned start, then extract full 8-bit values.
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[0xff, 0xf0, 0xaa, 0xbb]);
        assert_eq!(bs.extract_single(4).unwrap() & 0x0f, 0x0f);

        let mask = 0xff;
        assert_eq!(bs.extract_single(8).unwrap() & mask, 0x0f);
        assert_eq!(bs.extract_single(8).unwrap() & mask, 0xaf);
        assert_eq!(bs.extract_single(8).unwrap() & mask, 0xba);
        assert_eq!(bs.available(), 4);
    }

    #[test]
    fn extract_all_unaligned_8bit() {
        // Unaligned start, then extract full 8-bit values.
        let mut bs = ByteStreamReadBuffer::new();
        append(&mut bs, &[0xff, 0xf0, 0xaa, 0xbb]);
        bs.extract_single(4).unwrap();

        let mut values = Vec::new();
        bs.extract_all(8, |v| values.push(v));

        assert_eq!(values, [0x0f, 0xaf, 0xba]);
        assert_eq!(bs.available(), 4);
    }
}
