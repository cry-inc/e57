use crate::error::Converter;
use crate::{Error, Result};
use std::io::{Read, Seek, SeekFrom, Write};

#[cfg(not(feature = "crc32c"))]
use crate::crc32::Crc32;

const PAGE_SIZE: u64 = 1024;
const CRC_SIZE: u64 = 4;
const PAGE_PAYLOAD_SIZE: usize = (PAGE_SIZE - CRC_SIZE) as usize;

pub struct PagedWriter<T: Write + Read + Seek> {
    writer: T,
    offset: usize,
    page_buffer: [u8; PAGE_SIZE as usize],

    #[cfg(not(feature = "crc32c"))]
    crc: Crc32,
}

impl<T: Write + Read + Seek> PagedWriter<T> {
    /// Create and initialize a paged writer that abstracts the E57 CRC scheme
    pub fn new(mut writer: T) -> Result<Self> {
        let end = writer
            .seek(SeekFrom::End(0))
            .read_err("Unable to seek length of writer")?;
        if end != 0 {
            Err(Error::Write {
                desc: String::from("Supplied writer is not empty"),
                source: None,
            })?
        }
        Ok(Self {
            writer,
            offset: 0,
            page_buffer: [0_u8; PAGE_SIZE as usize],

            #[cfg(not(feature = "crc32c"))]
            crc: Crc32::new(),
        })
    }

    /// Get the current physical offset in the file.
    pub fn physical_position(&mut self) -> Result<u64> {
        let pos = self
            .writer
            .stream_position()
            .read_err("Failed to get position from writer")?;
        Ok(pos + self.offset as u64)
    }

    /// Seek to a specific physical offset in the file.
    pub fn physical_seek(&mut self, pos: u64) -> Result<()> {
        // Make sure we wrote the current (partial) page before seeking
        self.flush().write_err("Failed to flush before seeking")?;

        let end = self
            .writer
            .seek(SeekFrom::End(0))
            .write_err("Failed to seek to file end")?;
        let page = pos / PAGE_SIZE;
        self.offset = (pos % PAGE_SIZE) as usize;

        if pos > end {
            Err(Error::Write {
                desc: String::from("Cannot seek after end of file"),
                source: None,
            })?
        }
        if self.offset >= PAGE_PAYLOAD_SIZE {
            Err(Error::Write {
                desc: String::from("Cannot seek into checksum"),
                source: None,
            })?
        }

        let page_phys_offset = page * PAGE_SIZE;
        self.writer
            .seek(SeekFrom::Start(page_phys_offset))
            .write_err("Failed to seek to specified position")?;

        self.populate_existing_data()
            .write_err("Failed to read existing page data")?;

        self.writer
            .seek(SeekFrom::Start(page_phys_offset))
            .write_err("Failed to seek back to page start after reading existing data")?;

        Ok(())
    }

    fn populate_existing_data(&mut self) -> std::io::Result<()> {
        // If available, read existing page data
        let mut unread = &mut self.page_buffer[..];
        while !unread.is_empty() {
            let read = self.writer.read(unread)?;
            if read == 0 {
                break;
            }
            unread = &mut unread[read..];
        }
        unread.fill(0);
        Ok(())
    }

    // Get the current physical size of the file.
    pub fn physical_size(&mut self) -> Result<u64> {
        self.flush().write_err("Cannot flush writer")?;
        let pos = self
            .writer
            .stream_position()
            .write_err("Cannot get current position")?;
        let size = self
            .writer
            .seek(SeekFrom::End(0))
            .write_err("Cannot seek to file end")?;
        self.writer
            .seek(SeekFrom::Start(pos))
            .write_err("Cannot seek to previous position")?;
        Ok(size)
    }

    /// Write some zeros to next 4-byte-aligned offset, if needed.
    pub fn align(&mut self) -> Result<()> {
        let zeros = [0u8; 4];
        let mod_offset = self.offset % 4;
        if mod_offset != 0 {
            self.write_all(&zeros[mod_offset..])
                .write_err("Failed to write zero bytes for alignment")?;
        }
        Ok(())
    }
}

impl<T: Write + Read + Seek> Write for PagedWriter<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining_page_bytes = PAGE_PAYLOAD_SIZE - self.offset;
        let writeable_bytes = buf.len().min(remaining_page_bytes);
        self.page_buffer[self.offset..self.offset + writeable_bytes]
            .copy_from_slice(&buf[..writeable_bytes]);
        self.offset += writeable_bytes;
        if self.offset == PAGE_PAYLOAD_SIZE {
            // Simple & slower default included SW implementation
            #[cfg(not(feature = "crc32c"))]
            let crc = self.crc.calculate(&self.page_buffer[..PAGE_PAYLOAD_SIZE]);

            // Optional faster external crate with HW support
            #[cfg(feature = "crc32c")]
            let crc = crc32c::crc32c(&self.page_buffer[..PAGE_PAYLOAD_SIZE]);

            self.page_buffer[PAGE_PAYLOAD_SIZE..].copy_from_slice(&crc.to_be_bytes());
            self.writer.write_all(&self.page_buffer)?;

            let page_phys_offset = self.writer.stream_position()?;
            self.offset = 0;
            self.populate_existing_data()?;
            self.writer.seek(SeekFrom::Start(page_phys_offset))?;
        }
        Ok(writeable_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // If the page buffer is empty we do not need to persist it
        if self.offset > 0 {
            // Store start position in current page
            let pos = self.writer.stream_position()?;

            // Simple & slower default included SW implementation
            #[cfg(not(feature = "crc32c"))]
            let crc = self.crc.calculate(&self.page_buffer[..PAGE_PAYLOAD_SIZE]);

            // Optional faster external crate with HW support
            #[cfg(feature = "crc32c")]
            let crc = crc32c::crc32c(&self.page_buffer[..PAGE_PAYLOAD_SIZE]);

            // Write current page
            self.page_buffer[PAGE_PAYLOAD_SIZE..].copy_from_slice(&crc.to_be_bytes());
            self.writer.write_all(&self.page_buffer)?;

            // Seek back to start position
            self.writer.seek(SeekFrom::Start(pos))?;
        }

        // Forward flush to underlying writer
        self.writer.flush()
    }
}

impl<T: Write + Read + Seek> Drop for PagedWriter<T> {
    fn drop(&mut self) {
        if self.flush().is_err() {
            // Cannot handle the error here :/
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, OpenOptions};
    use std::path::Path;

    fn open_options() -> OpenOptions {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(true);
        options
    }

    #[test]
    fn empty() {
        let path = Path::new("empty.bin");
        let file = open_options().open(path).unwrap();
        let writer = PagedWriter::new(file).unwrap();
        drop(writer);
        assert_eq!(path.metadata().unwrap().len(), 0);
        remove_file(path).unwrap();
    }

    #[test]
    fn partial_page() {
        let path = Path::new("partial.bin");
        let file = open_options().open(path).unwrap();

        // Write only three bytes
        let mut writer = PagedWriter::new(file).unwrap();
        writer.write_all(&[0_u8, 1_u8, 2_u8]).unwrap();
        drop(writer);
        assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

        // Check file content
        let content = std::fs::read(path).unwrap();
        assert_eq!(content[0], 0_u8);
        assert_eq!(content[1], 1_u8);
        assert_eq!(content[2], 2_u8);
        for i in 3..PAGE_PAYLOAD_SIZE {
            assert_eq!(content[i], 0_u8);
        }
        assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[156, 69, 208, 231]);

        remove_file(path).unwrap();
    }

    #[test]
    fn single_page() {
        let path = Path::new("single.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Write exactly one page
        let data = vec![1_u8; PAGE_PAYLOAD_SIZE];
        writer.write_all(&data).unwrap();
        drop(writer);
        assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

        // Check file content
        let content = std::fs::read(path).unwrap();
        for i in 0..PAGE_PAYLOAD_SIZE {
            assert_eq!(content[i], 1_u8);
        }
        assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[25, 85, 144, 35]);

        remove_file(path).unwrap();
    }

    #[test]
    fn multi_page() {
        let path = Path::new("multi.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Write a little bit more than one page
        let mut data = vec![1_u8; PAGE_PAYLOAD_SIZE + 1];
        data[PAGE_PAYLOAD_SIZE] = 2_u8;
        writer.write_all(&data).unwrap();
        drop(writer);
        assert_eq!(path.metadata().unwrap().len(), 2 * PAGE_SIZE);

        // Load file content
        let content = std::fs::read(path).unwrap();

        // Check first page with ones
        let offset = 0;
        for i in 0..PAGE_PAYLOAD_SIZE {
            assert_eq!(content[offset + i], 1_u8);
        }
        assert_eq!(
            &content[PAGE_PAYLOAD_SIZE..PAGE_PAYLOAD_SIZE + CRC_SIZE as usize],
            &[25, 85, 144, 35]
        );

        // Check second page with one two and lots of zeros
        let offset = PAGE_SIZE as usize;
        assert_eq!(content[offset], 2_u8);
        for i in 1..PAGE_PAYLOAD_SIZE {
            assert_eq!(content[offset + i], 0_u8);
        }
        assert_eq!(
            &content[(offset + PAGE_PAYLOAD_SIZE)..],
            &[40, 41, 250, 169]
        );

        remove_file(path).unwrap();
    }

    #[test]
    fn flush_in_page() {
        let path = Path::new("flush.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Partial page
        writer.write_all(&[0_u8, 1_u8, 2_u8]).unwrap();

        // Flush
        writer.flush().unwrap();

        // Write more data into page
        writer.write_all(&[3_u8, 4_u8, 5_u8]).unwrap();

        // Close and check size
        drop(writer);
        assert_eq!(path.metadata().unwrap().len(), PAGE_SIZE);

        // Check file content
        let content = std::fs::read(path).unwrap();
        for i in 0..6 {
            assert_eq!(content[i], i as u8);
        }
        for i in 6..PAGE_PAYLOAD_SIZE {
            assert_eq!(content[i], 0_u8);
        }
        assert_eq!(&content[PAGE_PAYLOAD_SIZE..], &[50, 14, 64, 153]);

        remove_file(path).unwrap();
    }

    #[test]
    fn seek_existing_page() {
        let path = Path::new("seek_existing.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Write two pages with ones
        let data = vec![1_u8; PAGE_PAYLOAD_SIZE * 2];
        writer.write_all(&data).unwrap();

        // Got back to start and write some twos
        writer.physical_seek(2).unwrap();
        writer.write_all(&[2_u8, 2_u8]).unwrap();
        drop(writer);

        // Check file content
        let content = std::fs::read(path).unwrap();
        assert_eq!(content[0], 1_u8);
        assert_eq!(content[1], 1_u8);
        assert_eq!(content[2], 2_u8);
        assert_eq!(content[3], 2_u8);
        assert_eq!(content[4], 1_u8);
        assert_eq!(content[5], 1_u8);

        remove_file(path).unwrap();
    }

    #[test]
    fn seek_after_end() {
        let path = Path::new("seek_after_end.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Seek to start should work
        writer.physical_seek(0).unwrap();

        // Seeking further fails
        assert!(writer.physical_seek(2).is_err());

        remove_file(path).unwrap();
    }

    #[test]
    fn phys_position_size() {
        let path = Path::new("phys_position_size.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        // Write a page and some bytes
        let data = vec![1_u8; 1028];
        writer.write_all(&data).unwrap();

        // We expect the physical position to be the logical + CRC size
        let pos = writer.physical_position().unwrap();
        assert_eq!(pos, 1028 + CRC_SIZE);

        // We expect the physical size to be two pages with CRC sums
        let size = writer.physical_size().unwrap();
        assert_eq!(size, PAGE_SIZE * 2);

        remove_file(path).unwrap();
    }

    #[test]
    fn align() {
        let path = Path::new("align.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        writer.align().unwrap();
        assert_eq!(writer.physical_position().unwrap(), 0);

        let data = vec![1_u8; 2];
        writer.write_all(&data).unwrap();
        writer.align().unwrap();
        assert_eq!(writer.physical_position().unwrap(), 4);

        // Check file content
        drop(writer);
        let content = std::fs::read(path).unwrap();
        assert_eq!(content[0], 1_u8);
        assert_eq!(content[1], 1_u8);
        assert_eq!(content[2], 0_u8);
        assert_eq!(content[3], 0_u8);

        remove_file(path).unwrap();
    }

    #[test]
    fn short_seek_back() {
        let path = Path::new("short_seek_back.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        writer.write_all(&[4, 1, 2, 3]).unwrap();
        // This seek places the cursor within an incomplete page.
        writer.physical_seek(0).unwrap();
        writer.write_all(&[0]).unwrap();
        writer.flush().unwrap();

        // Check file content
        drop(writer);
        let content = std::fs::read(path).unwrap();
        assert_eq!(content[0], 0_u8);
        assert_eq!(content[1], 1_u8);
        assert_eq!(content[2], 2_u8);
        assert_eq!(content[3], 3_u8);

        remove_file(path).unwrap();
    }

    #[test]
    fn write_into_page() {
        let path = Path::new("write_into_page.bin");
        let file = open_options().open(path).unwrap();
        let mut writer = PagedWriter::new(file).unwrap();

        writer.write_all(&[1; PAGE_PAYLOAD_SIZE]).unwrap();
        writer.write_all(&[2; PAGE_PAYLOAD_SIZE]).unwrap();
        writer.physical_seek(PAGE_PAYLOAD_SIZE as u64 - 1).unwrap();
        // These two bytes span a page boundary.
        writer.write_all(&[3; 2]).unwrap();
        writer.flush().unwrap();

        // Check file content
        drop(writer);
        let content = std::fs::read(path).unwrap();
        for i in 0..PAGE_PAYLOAD_SIZE - 1 {
            assert_eq!(
                content[i], 1_u8,
                "Expected 1 at {} but got {}",
                i, content[i],
            );
        }
        assert_eq!(3, content[PAGE_PAYLOAD_SIZE - 1]);
        assert_eq!(3, content[PAGE_SIZE as usize]);
        for i in PAGE_SIZE as usize + 1..(PAGE_SIZE as usize + PAGE_PAYLOAD_SIZE) {
            assert_eq!(
                content[i], 2_u8,
                "Expected 2 at {} but got {}",
                i, content[i],
            );
        }

        remove_file(path).unwrap();
    }
}
