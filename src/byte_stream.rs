#[derive(Clone)]
pub struct ByteStream {
    buffer: Vec<u8>,
    offset: u64,
}

pub struct ByteStreamExtraction {
    pub data: Vec<u8>,
    pub bits: u64,
    pub offset: u64,
}

impl ByteStream {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
        }
    }

    pub fn append(&mut self, mut data: Vec<u8>) {
        let bytes_to_remove = (self.offset / 8) as usize;
        if bytes_to_remove > 0 {
            self.buffer = self.buffer[bytes_to_remove..].to_vec();
            self.offset -= bytes_to_remove as u64 * 8;
        }
        self.buffer.append(&mut data);
    }

    pub fn extract(&mut self, bits: u64) -> Option<ByteStreamExtraction> {
        if self.available() >= bits {
            let start_offset = (self.offset / 8) as usize;
            let end_offset = ((self.offset + bits) as f32 / 8.).ceil() as usize;
            let offset = self.offset % 8;
            let data = self.buffer[start_offset..end_offset].to_vec();
            self.offset += bits;
            Some(ByteStreamExtraction { data, bits, offset })
        } else {
            None
        }
    }

    pub fn available(&self) -> u64 {
        (self.buffer.len() as u64 * 8) - self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut bs = ByteStream::new();
        assert_eq!(bs.available(), 0);
        let result = bs.extract(0).unwrap();
        assert_eq!(result.bits, 0);
        assert_eq!(result.offset, 0);
        assert_eq!(result.data, Vec::new());

        assert_eq!(bs.available(), 0);
        assert!(bs.extract(1).is_none());
    }

    #[test]
    fn append_and_extract_bits() {
        let mut bs = ByteStream::new();
        bs.append(vec![255]);

        assert_eq!(bs.available(), 8);
        let result = bs.extract(2).unwrap();
        assert_eq!(result.bits, 2);
        assert_eq!(result.offset, 0);
        assert_eq!(result.data, vec![255_u8]);

        assert_eq!(bs.available(), 6);
        let result = bs.extract(6).unwrap();
        assert_eq!(result.bits, 6);
        assert_eq!(result.offset, 2);
        assert_eq!(result.data, vec![255]);

        assert_eq!(bs.available(), 0);
        assert!(bs.extract(1).is_none());
    }

    #[test]
    fn append_and_extract_bytes() {
        let mut bs = ByteStream::new();
        bs.append(vec![23, 42, 13]);
        bs.extract(2).unwrap();

        assert_eq!(bs.available(), 22);
        let result = bs.extract(22).unwrap();
        assert_eq!(result.bits, 22);
        assert_eq!(result.offset, 2);
        assert_eq!(result.data, vec![23, 42, 13]);
    }

    #[test]
    fn remove_consume_when_appending() {
        let mut bs = ByteStream::new();
        bs.append(vec![1, 2, 3, 4, 5]);
        bs.extract(4 * 8 + 2).unwrap();

        // We append one byte and the buffer should become smaller
        // because all fully consumed bytes are removed.
        bs.append(vec![6]);
        assert!(bs.buffer.len() == 2);

        // Offsets are updated correctly appended
        // data can be extracted as expected.
        let result = bs.extract(14).unwrap();
        assert_eq!(result.bits, 14);
        assert_eq!(result.offset, 2);
        assert_eq!(result.data, vec![5, 6]);
    }
}
