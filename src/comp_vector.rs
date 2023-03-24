use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::Result;
use std::io::Write;
use std::io::{Read, Seek};

#[derive(Debug)]
pub struct CompressedVectorSectionHeader {
    section_id: u8,
    pub section_length: u64,
    pub data_offset: u64,
    pub index_offset: u64,
}

impl CompressedVectorSectionHeader {
    pub const SIZE: u64 = 32;

    pub fn read(reader: &mut dyn Read) -> Result<CompressedVectorSectionHeader> {
        let mut buffer = [0_u8; Self::SIZE as usize];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read compressed vector section header")?;

        let header = Self {
            section_id: buffer[0],
            section_length: u64::from_le_bytes(
                buffer[8..16].try_into().internal_err(WRONG_OFFSET)?,
            ),
            data_offset: u64::from_le_bytes(buffer[16..24].try_into().internal_err(WRONG_OFFSET)?),
            index_offset: u64::from_le_bytes(buffer[24..32].try_into().internal_err(WRONG_OFFSET)?),
        };

        if header.section_id != 1 {
            Error::invalid("Section ID of the compressed vector section header is not 1")?
        }
        if header.section_length % 4 != 0 {
            Error::invalid("Section length is not aligned and a multiple of four")?
        }

        Ok(header)
    }

    pub fn write(&self, writer: &mut dyn Write) -> Result<()> {
        let mut buffer = [0_u8; Self::SIZE as usize];
        buffer[0] = self.section_id;
        buffer[8..16].copy_from_slice(&self.section_length.to_le_bytes());
        buffer[16..24].copy_from_slice(&self.data_offset.to_le_bytes());
        buffer[24..32].copy_from_slice(&self.index_offset.to_le_bytes());
        writer
            .write_all(&buffer)
            .write_err("Failed to write compressed vector section header")
    }
}

impl Default for CompressedVectorSectionHeader {
    fn default() -> Self {
        Self {
            section_id: 1,
            section_length: 0,
            data_offset: 0,
            index_offset: 0,
        }
    }
}

#[derive(Debug)]
pub enum PacketHeader {
    Index {
        packet_length: u64,
        entry_count: u16,
        index_level: u8,
    },
    Data {
        packet_flags: PacketFlags,
        packet_length: u64,
        bytestream_count: u16,
    },
    Ignored {
        packet_length: u64,
    },
}

impl PacketHeader {
    pub fn from_reader<T: Read + Seek>(reader: &mut PagedReader<T>) -> Result<Self> {
        // Read only first byte of header to indetify packet type
        let mut buffer = [0_u8; 1];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read packet type")?;

        if buffer[0] == 0 {
            // Read Index Packet
            let mut buffer = [0_u8; 15];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read index packet header")?;

            // Parse values
            let packet_length =
                u16::from_le_bytes(buffer[1..3].try_into().internal_err(WRONG_OFFSET)?) as u64 + 1;
            let entry_count =
                u16::from_le_bytes(buffer[3..5].try_into().internal_err(WRONG_OFFSET)?);
            let index_level = buffer[5];

            // Validate values
            if packet_length == 0 {
                Error::invalid("A data packet length of 0 is not allowed")?
            }
            if packet_length % 4 != 0 {
                Error::invalid("Index packet length is not aligned and a multiple of four")?
            }

            Ok(PacketHeader::Index {
                packet_length,
                entry_count,
                index_level,
            })
        } else if buffer[0] == 1 {
            // Read Data Packet
            let mut buffer = [0_u8; 5];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read data packet header")?;

            // Parse values
            let packet_flags = PacketFlags::from_byte(buffer[0]);
            let packet_length =
                u16::from_le_bytes(buffer[1..3].try_into().internal_err(WRONG_OFFSET)?) as u64 + 1;
            let bytestream_count =
                u16::from_le_bytes(buffer[3..5].try_into().internal_err(WRONG_OFFSET)?);

            // Validate values
            if packet_length == 0 {
                Error::invalid("A data packet length of 0 is not allowed")?
            }
            if packet_length % 4 != 0 {
                Error::invalid("Data packet length is not aligned and a multiple of four")?
            }
            if bytestream_count == 0 {
                Error::invalid("A byte stream count of 0 is not allowed")?
            }

            Ok(PacketHeader::Data {
                packet_flags,
                packet_length,
                bytestream_count,
            })
        } else if buffer[0] == 2 {
            // Read Ignored Packet
            let mut buffer = [0_u8; 3];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read ignore packet header")?;

            // Parse values
            let packet_length =
                u16::from_le_bytes(buffer[1..3].try_into().internal_err(WRONG_OFFSET)?) as u64 + 1;

            // Validate values
            if packet_length == 0 {
                Error::invalid("A ignored packet length of 0 is not allowed")?
            }
            if packet_length % 4 != 0 {
                Error::invalid("Ignored packet length is not aligned and a multiple of four")?
            }

            Ok(PacketHeader::Ignored { packet_length })
        } else {
            Error::invalid("Found unknown packet ID when trying to read packet header")?
        }
    }
}

#[derive(Debug)]
pub struct PacketFlags {
    _compressor_restart: bool,
}

impl PacketFlags {
    pub fn from_byte(value: u8) -> Self {
        Self {
            _compressor_restart: value & 1 != 0,
        }
    }
}
