#[derive(Clone)]
pub struct ByteStreamReadBuffer {
    buffer: Vec<u8>,
    offset: u64,
}

pub struct ByteStreamData {
    pub data: [u8; 8],
    pub data_len: u8,
    pub offset: u8,
}

impl ByteStreamReadBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
        }
    }

    pub fn append(&mut self, data: &[u8]) {
        let bytes_to_remove = (self.offset / 8) as usize;
        if bytes_to_remove > 0 {
            self.buffer = self.buffer[bytes_to_remove..].to_vec();
            self.offset -= bytes_to_remove as u64 * 8;
        }
        self.buffer.extend_from_slice(data);
    }

    pub fn extract(&mut self, bits: u64) -> Option<ByteStreamData> {
        if self.available() >= bits {
            let start_offset = (self.offset / 8) as usize;
            let end_offset = ((self.offset + bits) as f32 / 8.).ceil() as usize;
            let offset = self.offset % 8;
            let mut data = [0; 8];
            let data_len = end_offset - start_offset;
            let dst = &mut data[..data_len];
            let src = &self.buffer[start_offset..end_offset];
            dst.copy_from_slice(src);
            self.offset += bits;
            let data_len = data_len as u8;
            let offset = offset as u8;
            Some(ByteStreamData {
                data,
                data_len,
                offset,
            })
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
        let mut bs = ByteStreamReadBuffer::new();
        assert_eq!(bs.available(), 0);
        let result = bs.extract(0).unwrap();
        assert_eq!(result.offset, 0);
        assert_eq!(result.data_len, 0);
        assert_eq!(result.data, [0, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(bs.available(), 0);
        assert!(bs.extract(1).is_none());
    }

    #[test]
    fn append_and_extract_bits() {
        let mut bs = ByteStreamReadBuffer::new();
        bs.append(&[255]);

        assert_eq!(bs.available(), 8);
        let result = bs.extract(2).unwrap();
        assert_eq!(result.offset, 0);
        assert_eq!(result.data_len, 1);
        assert_eq!(result.data, [255, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(bs.available(), 6);
        let result = bs.extract(6).unwrap();
        assert_eq!(result.offset, 2);
        assert_eq!(result.data_len, 1);
        assert_eq!(result.data, [255, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(bs.available(), 0);
        assert!(bs.extract(1).is_none());
    }

    #[test]
    fn append_and_extract_bytes() {
        let mut bs = ByteStreamReadBuffer::new();
        bs.append(&[23, 42, 13]);
        bs.extract(2).unwrap();

        assert_eq!(bs.available(), 22);
        let result = bs.extract(22).unwrap();
        assert_eq!(result.offset, 2);
        assert_eq!(result.data_len, 3);
        assert_eq!(result.data, [23, 42, 13, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn remove_consume_when_appending() {
        let mut bs = ByteStreamReadBuffer::new();
        bs.append(&[1, 2, 3, 4, 5]);
        bs.extract(4 * 8 + 2).unwrap();

        // We append one byte and the buffer should become smaller
        // because all fully consumed bytes are removed.
        bs.append(&[6]);
        assert!(bs.buffer.len() == 2);

        // Offsets are updated correctly appended
        // data can be extracted as expected.
        let result = bs.extract(14).unwrap();
        assert_eq!(result.offset, 2);
        assert_eq!(result.data_len, 2);
        assert_eq!(result.data, [5, 6, 0, 0, 0, 0, 0, 0]);
    }
}
