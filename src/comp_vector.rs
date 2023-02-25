use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::paged_reader::PagedReader;
use crate::Error;
use crate::Result;
use std::io::{Read, Seek};

#[derive(Debug)]
pub struct CompressedVectorHeader {
    pub _section_length: u64,
    pub data_start_offset: u64,
    pub _index_start_offset: u64,
}

impl CompressedVectorHeader {
    pub fn from_array(buffer: &[u8; 32]) -> Result<Self> {
        if buffer[0] != 1 {
            Error::invalid("Section ID of the compressed vector section header is not 1")?
        }
        Ok(Self {
            _section_length: u64::from_le_bytes(
                buffer[8..16].try_into().internal_err(WRONG_OFFSET)?,
            ),
            data_start_offset: u64::from_le_bytes(
                buffer[16..24].try_into().internal_err(WRONG_OFFSET)?,
            ),
            _index_start_offset: u64::from_le_bytes(
                buffer[24..32].try_into().internal_err(WRONG_OFFSET)?,
            ),
        })
    }

    pub fn from_reader<T: Read + Seek>(
        reader: &mut PagedReader<T>,
    ) -> Result<CompressedVectorHeader> {
        let mut buffer = [0_u8; 32];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read compressed vector section header")?;
        CompressedVectorHeader::from_array(&buffer)
    }
}

#[derive(Debug)]
pub enum PacketHeader {
    Index {
        _packet_length: u32,
        _entry_count: u16,
        _index_level: u8,
    },
    Data {
        _packet_flags: PacketFlags,
        _packet_length: u32,
        bytestream_count: u16,
    },
    Ignored {
        _packet_length: u32,
    },
}

impl PacketHeader {
    pub fn from_reader<T: Read + Seek>(reader: &mut PagedReader<T>) -> Result<Self> {
        let mut buffer = [0_u8; 1];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read packet type")?;
        if buffer[0] == 0 {
            // Index Packet
            let mut buffer = [0_u8; 15];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read index packet header")?;
            Ok(PacketHeader::Index {
                _packet_length: u16::from_le_bytes(
                    buffer[1..3].try_into().internal_err(WRONG_OFFSET)?,
                ) as u32
                    + 1,
                _entry_count: u16::from_le_bytes(
                    buffer[3..5].try_into().internal_err(WRONG_OFFSET)?,
                ),
                _index_level: buffer[5],
            })
        } else if buffer[0] == 1 {
            // Data Packet
            let mut buffer = [0_u8; 5];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read data packet header")?;
            Ok(PacketHeader::Data {
                _packet_flags: PacketFlags::from_byte(buffer[0]),
                _packet_length: u16::from_le_bytes(
                    buffer[1..3].try_into().internal_err(WRONG_OFFSET)?,
                ) as u32
                    + 1,
                bytestream_count: u16::from_le_bytes(
                    buffer[3..5].try_into().internal_err(WRONG_OFFSET)?,
                ),
            })
        } else if buffer[0] == 2 {
            // Ignored Packet
            let mut buffer = [0_u8; 3];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read ignore packet header")?;
            Ok(PacketHeader::Ignored {
                _packet_length: u16::from_le_bytes(
                    buffer[1..3].try_into().internal_err(WRONG_OFFSET)?,
                ) as u32
                    + 1,
            })
        } else {
            Error::invalid("Found unknown packet ID when trying to read packet header")?
        }
    }
}

#[derive(Debug)]
pub struct PacketFlags {
    pub _compressor_restart: bool,
}

impl PacketFlags {
    pub fn from_byte(value: u8) -> Self {
        Self {
            _compressor_restart: value & 1 != 0,
        }
    }
}
