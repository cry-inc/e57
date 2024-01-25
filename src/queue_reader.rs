use crate::bitpack::BitPack;
use crate::bs_read::ByteStreamReadBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::PacketHeader;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::PointCloud;
use crate::RawValues;
use crate::RecordDataType;
use crate::RecordValue;
use crate::Result;
use std::collections::VecDeque;
use std::io::{Read, Seek};

/// Read compressed vector sections into queues of raw values.
pub struct QueueReader<'a, T: Read + Seek> {
    pc: PointCloud,
    reader: &'a mut PagedReader<T>,
    buffer: Vec<u8>,
    buffer_sizes: Vec<usize>,
    byte_streams: Vec<ByteStreamReadBuffer>,
    queues: Vec<VecDeque<RecordValue>>,
}

impl<'a, T: Read + Seek> QueueReader<'a, T> {
    pub fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        reader
            .seek_physical(pc.file_offset)
            .read_err("Cannot seek to compressed vector header")?;
        let section_header = CompressedVectorSectionHeader::read(reader)?;
        reader
            .seek_physical(section_header.data_offset)
            .read_err("Cannot seek to packet header")?;

        Ok(Self {
            pc: pc.clone(),
            reader,
            buffer: Vec::new(),
            buffer_sizes: vec![0; pc.prototype.len()],
            byte_streams: vec![ByteStreamReadBuffer::new(); pc.prototype.len()],
            queues: vec![VecDeque::new(); pc.prototype.len()],
        })
    }

    /// Returns the number of complete and available points across all queues.
    pub fn available(&self) -> usize {
        if self.queues.is_empty() {
            return 0;
        }

        let mut av = usize::MAX;
        for q in &self.queues {
            let len = q.len();
            if len < av {
                av = len;
            }
        }
        av
    }

    /// Return values for the next point by popping one value from each queue.
    /// Use an existing vector with enough capacity to avoid frequent reallocations!
    pub fn pop_point(&mut self, output: &mut RawValues) -> Result<()> {
        output.clear();
        for i in 0..self.pc.prototype.len() {
            let value = self.queues[i]
                .pop_front()
                .internal_err("Failed to pop value for next point")?;
            output.push(value);
        }
        Ok(())
    }

    /// Reads the next packet from the compressed vector and decodes it into the queues.
    pub fn advance(&mut self) -> Result<()> {
        let packet_header = PacketHeader::read(self.reader)?;
        match packet_header {
            PacketHeader::Index(_) => {
                Error::not_implemented("Index packets are not yet supported")?
            }
            PacketHeader::Ignored(_) => {
                Error::not_implemented("Ignored packets are not yet supported")?
            }
            PacketHeader::Data(header) => {
                if header.bytestream_count as usize != self.byte_streams.len() {
                    Error::invalid("Bytestream count does not match prototype size")?
                }

                // Read byte stream sizes
                for i in 0..self.buffer_sizes.len() {
                    let mut buf = [0_u8; 2];
                    self.reader
                        .read_exact(&mut buf)
                        .read_err("Failed to read data packet buffer sizes")?;
                    let len = u16::from_le_bytes(buf) as usize;
                    self.buffer_sizes[i] = len;
                }

                // Read byte streams into memory
                for (i, bs) in self.buffer_sizes.iter().enumerate() {
                    self.buffer.resize(*bs, 0_u8);
                    self.reader
                        .read_exact(&mut self.buffer)
                        .read_err("Failed to read data packet buffers")?;
                    self.byte_streams[i].append(&self.buffer);
                }

                self.parse_byte_streams()?;
            }
        };

        self.reader
            .align()
            .read_err("Failed to align reader on next 4-byte offset after reading packet")
    }

    /// Extracts raw values from byte streams into queues.
    fn parse_byte_streams(&mut self) -> Result<()> {
        for (i, r) in self.pc.prototype.iter().enumerate() {
            match r.data_type {
                RecordDataType::Single { .. } => {
                    BitPack::unpack_singles(&mut self.byte_streams[i], &mut self.queues[i])?
                }
                RecordDataType::Double { .. } => {
                    BitPack::unpack_doubles(&mut self.byte_streams[i], &mut self.queues[i])?
                }
                RecordDataType::ScaledInteger { min, max, .. } => BitPack::unpack_scaled_ints(
                    &mut self.byte_streams[i],
                    min,
                    max,
                    &mut self.queues[i],
                )?,
                RecordDataType::Integer { min, max } => {
                    BitPack::unpack_ints(&mut self.byte_streams[i], min, max, &mut self.queues[i])?
                }
            };
        }

        Ok(())
    }
}
