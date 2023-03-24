use crate::error::{Converter, WRONG_OFFSET};
use crate::{Error, Result};
use std::io::{Read, Write};

pub enum PacketHeader {
    Index(IndexPacketHeader),
    Data(DataPacketHeader),
    Ignored(IgnoredPacketHeader),
}

impl PacketHeader {
    pub fn read(reader: &mut dyn Read) -> Result<Self> {
        // Read only first byte of header to indetify packet type
        let mut buffer = [0_u8; 1];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read packet type ID")?;

        if buffer[0] == 0 {
            Ok(PacketHeader::Index(IndexPacketHeader::read(reader)?))
        } else if buffer[0] == 1 {
            Ok(PacketHeader::Data(DataPacketHeader::read(reader)?))
        } else if buffer[0] == 2 {
            Ok(PacketHeader::Ignored(IgnoredPacketHeader::read(reader)?))
        } else {
            Error::invalid("Found unknown packet ID when trying to read packet header")?
        }
    }
}

pub struct IndexPacketHeader {
    pub packet_length: u64,
    pub entry_count: u16,
    pub index_level: u8,
}

impl IndexPacketHeader {
    pub fn read(reader: &mut dyn Read) -> Result<Self> {
        let mut buffer = [0_u8; 15];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read index packet header")?;

        // Parse values
        let packet_length =
            u16::from_le_bytes(buffer[1..3].try_into().internal_err(WRONG_OFFSET)?) as u64 + 1;
        let entry_count = u16::from_le_bytes(buffer[3..5].try_into().internal_err(WRONG_OFFSET)?);
        let index_level = buffer[5];

        // Validate values
        if packet_length == 0 {
            Error::invalid("A data packet length of 0 is not allowed")?
        }
        if packet_length % 4 != 0 {
            Error::invalid("Index packet length is not aligned and a multiple of four")?
        }

        Ok(Self {
            packet_length,
            entry_count,
            index_level,
        })
    }
}

pub struct DataPacketHeader {
    pub comp_restart_flag: bool,
    pub packet_length: u64,
    pub bytestream_count: u16,
}

impl DataPacketHeader {
    pub fn read(reader: &mut dyn Read) -> Result<Self> {
        let mut buffer = [0_u8; 5];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read data packet header")?;

        // Parse values
        let comp_restart_flag = buffer[0] & 1 != 0;
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

        Ok(Self {
            comp_restart_flag,
            packet_length,
            bytestream_count,
        })
    }

    pub fn write(&self, writer: &mut dyn Write) -> Result<()> {
        let mut buffer = [0_u8; 6];
        buffer[0] = 1;
        let flags = if self.comp_restart_flag { 1_u8 } else { 0_u8 };
        buffer[1] = flags;
        let length = (self.packet_length - 1) as u16;
        buffer[2..4].copy_from_slice(&length.to_le_bytes());
        buffer[4..6].copy_from_slice(&self.bytestream_count.to_le_bytes());
        writer
            .write_all(&buffer)
            .write_err("Failed to write data packet header")
    }
}

pub struct IgnoredPacketHeader {
    pub packet_length: u64,
}

impl IgnoredPacketHeader {
    pub fn read(reader: &mut dyn Read) -> Result<Self> {
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

        Ok(Self { packet_length })
    }
}
