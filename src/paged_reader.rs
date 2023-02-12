use crc::{Crc, CRC_32_ISCSI};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};

const CHECKSUM_SIZE: u64 = CRC_32_ISCSI.width as u64 / 8;

pub struct PagedReader<T: Read + Seek> {
    page_size: u64,
    phy_file_size: u64,
    log_file_size: u64,
    pages: u64,
    reader: T,
    offset: u64,
    crc: Crc<u32>,
    page_num: Option<u64>,
    page_buffer: Vec<u8>,
}

impl<T: Read + Seek> PagedReader<T> {
    /// Create and initialize a paged reader that abstracts the E57 CRC scheme
    pub fn new(mut reader: T, page_size: u64) -> Result<Self> {
        if page_size <= CHECKSUM_SIZE {
            let msg = format!("Page size {page_size} needs to be bigger than checksum (4 bytes)");
            Err(Error::new(ErrorKind::InvalidInput, msg))?;
        }

        let phy_file_size = reader.seek(SeekFrom::End(0))?;
        if phy_file_size == 0 {
            let msg = "A file size of zero is not allowed";
            Err(Error::new(ErrorKind::InvalidData, msg))?;
        }
        if phy_file_size % page_size != 0 {
            let msg =
                format!("File size {phy_file_size} is not a multiple of the page size {page_size}");
            Err(Error::new(ErrorKind::InvalidData, msg))?;
        }

        let pages = phy_file_size / page_size;

        Ok(Self {
            reader,
            page_size,
            pages,
            phy_file_size,
            log_file_size: pages * (page_size - CHECKSUM_SIZE),
            crc: Crc::<u32>::new(&CRC_32_ISCSI),
            page_buffer: vec![0_u8; page_size as usize],
            page_num: None,
            offset: 0,
        })
    }

    /// Seeking to a physical file address as offset relative to the start of the file.
    /// Will return the new logical offset inside the file or an error.
    pub fn seek_physical(&mut self, offset: u64) -> Result<u64> {
        if offset >= self.phy_file_size {
            let msg = format!("Offset {offset} is behind end of file");
            Err(Error::new(ErrorKind::InvalidInput, msg))?;
        }

        let pages_before = offset / self.page_size;
        self.offset = offset - pages_before * CHECKSUM_SIZE;
        Ok(self.offset)
    }

    fn read_page(&mut self, page: u64) -> Result<()> {
        if page >= self.pages {
            let max = self.pages - 1;
            let msg = format!("Page {page} does not exist, only page numbers 0..{max} are valid");
            Err(Error::new(ErrorKind::InvalidInput, msg))?;
        }
        let offset = page * self.page_size;
        self.reader.seek(SeekFrom::Start(offset))?;
        self.reader.read_exact(&mut self.page_buffer)?;
        let data_size = self.page_size - CHECKSUM_SIZE;
        let mut digest = self.crc.digest();
        digest.update(&self.page_buffer[0..data_size as usize]);
        let expected_checksum = &self.page_buffer[data_size as usize..];

        // The standard says all binary values are stored as little endian,
        // but for some reason E57 files contain the checksum in big endian order.
        // Probably the reference implementation used a weird CRC library and
        // now everybody has to swap bytes as well because it was not noticed back then :)
        let calculated_checksum = digest.finalize().to_be_bytes();

        if expected_checksum != calculated_checksum {
            self.page_num = None;
            let msg = format!("Detected invalid checksum (expected: {expected_checksum:?}, actual: {calculated_checksum:?}) for page {page}");
            Err(Error::new(ErrorKind::InvalidData, msg))
        } else {
            self.page_num = Some(page);
            Ok(())
        }
    }

    /// Do some skipping to next 4-byte-aligned offset, if needed.
    /// Returns the new logical offset relative to the beginning of the file.
    pub fn align(&mut self) -> Result<u64> {
        let off_alignment = self.offset % 4;
        if off_alignment != 0 {
            let skip = 4 - off_alignment;
            if self.offset + skip > self.log_file_size {
                Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Tried to move behind end of file",
                ))?
            }
            self.offset += skip;
        }
        Ok(self.offset)
    }
}

impl<T: Read + Seek> Read for PagedReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let page = self.offset / (self.page_size - CHECKSUM_SIZE);
        if page >= self.pages {
            return Ok(0);
        }
        if self.page_num != Some(page) {
            self.read_page(page)?;
        }
        let page_offset = self.offset % (self.page_size - CHECKSUM_SIZE);
        let page_readable = self.page_size - CHECKSUM_SIZE - page_offset;
        let read_size = usize::min(buf.len(), page_readable as usize);
        buf[..read_size].copy_from_slice(
            &self.page_buffer[page_offset as usize..page_offset as usize + read_size],
        );
        self.offset += read_size as u64;
        Ok(read_size)
    }
}

impl<T: Read + Seek> Seek for PagedReader<T> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let new_offset = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::End(p) => (self.log_file_size as i64 + p) as u64,
            SeekFrom::Current(p) => (self.offset as i64 + p) as u64,
        };
        if new_offset > self.log_file_size {
            let msg = format!("Detected invalid offset {new_offset} after end of file");
            Err(Error::new(ErrorKind::InvalidInput, msg))?;
        }
        self.offset = new_offset;
        Ok(self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Cursor;

    const PAGE_SIZE: u64 = 1024;

    #[test]
    fn read_full_valid_file() {
        let file_size = 743424_u64;
        let pages = file_size / PAGE_SIZE;
        let logical_file_size = file_size - pages * CHECKSUM_SIZE;
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        let mut reader = PagedReader::new(file, PAGE_SIZE).unwrap();

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(buf.len(), logical_file_size as usize);
    }

    #[test]
    fn size_not_multiple_of_page() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        assert!(PagedReader::new(file, PAGE_SIZE - 1).is_err());
    }

    #[test]
    fn page_size_too_small() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        assert!(PagedReader::new(file, CHECKSUM_SIZE).is_err());
    }

    #[test]
    fn zero_pages() {
        let file = Vec::<u8>::new();
        let cursor = Cursor::new(file);
        assert!(PagedReader::new(cursor, PAGE_SIZE).is_err());
    }

    #[test]
    fn corrupt_page() {
        let data = vec![0_u8; 128];
        let cursor = Cursor::new(data);
        let mut reader = PagedReader::new(cursor, 128).unwrap();

        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn seek() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        let mut reader = PagedReader::new(file, PAGE_SIZE).unwrap();

        let xml_logical_offset = 737844;
        assert_eq!(
            reader.seek(SeekFrom::Start(xml_logical_offset)).unwrap(),
            xml_logical_offset
        );

        let mut buffer = [0_u8; 5];
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer.to_vec()).unwrap(), "<?xml");

        assert_eq!(reader.seek(SeekFrom::Start(0)).unwrap(), 0);

        let expected_logical_file_end = 740520;
        assert_eq!(
            reader.seek(SeekFrom::End(0)).unwrap(),
            expected_logical_file_end
        );

        assert_eq!(
            reader.seek(SeekFrom::Current(-10)).unwrap(),
            expected_logical_file_end - 10
        );
    }

    #[test]
    fn physical_seek() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        let mut reader = PagedReader::new(file, PAGE_SIZE).unwrap();

        let xml_physical_offset = 740736;
        let expected_logical_offset = 737844;

        let logical_offset = reader.seek_physical(xml_physical_offset).unwrap();
        assert_eq!(logical_offset, expected_logical_offset);

        let mut buffer = [0_u8; 5];
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer.to_vec()).unwrap(), "<?xml");
    }

    #[test]
    fn read_end() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        let mut reader = PagedReader::new(file, PAGE_SIZE).unwrap();

        reader.seek(SeekFrom::End(0)).unwrap();

        let mut buffer = Vec::new();
        assert_eq!(reader.read_to_end(&mut buffer).unwrap(), 0);
    }

    #[test]
    fn align() {
        let data = vec![0_u8; 128];
        let cursor = Cursor::new(data);
        let mut reader = PagedReader::new(cursor, 128).unwrap();

        let pos = reader.align().unwrap();
        assert_eq!(pos, 0);

        reader.seek(SeekFrom::Start(1)).unwrap();
        let pos = reader.align().unwrap();
        assert_eq!(pos, 4);

        let pos = reader.align().unwrap();
        assert_eq!(pos, 4);
    }
}
