#[derive(Clone)]
pub struct ByteStreamWriteBuffer {
    buffer: Vec<u8>,
    last_byte_bits: usize,
}

impl ByteStreamWriteBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            last_byte_bits: 8,
        }
    }

    pub fn add_bytes(&mut self, bytes: &[u8]) {
        if self.last_byte_bits == 8 {
            self.buffer.extend_from_slice(bytes);
        } else {
            todo!()
        }
    }

    pub fn get_full_bytes(&mut self) -> Vec<u8> {
        let to_take = self.full_bytes();
        self.buffer.drain(..to_take).collect()
    }

    pub fn full_bytes(&self) -> usize {
        let len = self.buffer.len();
        if len > 0 && self.last_byte_bits < 8 {
            len - 1
        } else {
            len
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut buffer = ByteStreamWriteBuffer::new();
        assert_eq!(buffer.full_bytes(), 0);

        let full = buffer.get_full_bytes();
        assert_eq!(full.len(), 0);
    }

    #[test]
    fn add_remove() {
        let mut buffer = ByteStreamWriteBuffer::new();
        buffer.add_bytes(&[1, 2, 3, 4]);
        assert_eq!(buffer.full_bytes(), 4);

        let extracted = buffer.get_full_bytes();
        assert_eq!(extracted.len(), 4);
        assert_eq!(buffer.full_bytes(), 0);
    }
}
