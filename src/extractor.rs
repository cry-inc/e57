use crate::bitpack::BitPack;
use crate::error::Converter;
use crate::paged_reader::PagedReader;
use crate::CartesianCoodinate;
use crate::Error;
use crate::PointCloud;
use crate::Record;
use crate::Result;
use std::io::{Read, Seek};

#[derive(Debug)]
struct CompressedVectorHeader {
    pub _section_length: u64,
    pub data_start_offset: u64,
    pub _index_start_offset: u64,
}

impl CompressedVectorHeader {
    pub fn from_array(buffer: &[u8; 32]) -> Result<Self> {
        if buffer[0] != 1 {
            Error::invalid("Section ID of the compressed vector header is not one")?
        }
        let msg = "Wrong header offsets detected";
        Ok(Self {
            _section_length: u64::from_le_bytes(buffer[8..16].try_into().internal_err(msg)?),
            data_start_offset: u64::from_le_bytes(buffer[16..24].try_into().internal_err(msg)?),
            _index_start_offset: u64::from_le_bytes(buffer[24..32].try_into().internal_err(msg)?),
        })
    }

    fn from_reader<T: Read + Seek>(reader: &mut PagedReader<T>) -> Result<CompressedVectorHeader> {
        let mut buffer = [0_u8; 32];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read compressed vector header")?;
        CompressedVectorHeader::from_array(&buffer)
    }
}

#[derive(Debug)]
enum PacketHeader {
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
    fn from_reader<T: Read + Seek>(reader: &mut PagedReader<T>) -> Result<Self> {
        let mut buffer = [0_u8; 1];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read packet type")?;
        if buffer[0] == 0 {
            let mut buffer = [0_u8; 15];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read index packet header")?;
            let msg = "Wrong header offsets detected";
            Ok(PacketHeader::Index {
                _packet_length: u16::from_le_bytes(buffer[1..3].try_into().internal_err(msg)?)
                    as u32
                    + 1,
                _entry_count: u16::from_le_bytes(buffer[3..5].try_into().internal_err(msg)?),
                _index_level: buffer[5],
            })
        } else if buffer[0] == 1 {
            let mut buffer = [0_u8; 5];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read data packet header")?;
            let msg = "Wrong header offsets detected";
            Ok(PacketHeader::Data {
                _packet_flags: PacketFlags::from_byte(buffer[0]),
                _packet_length: u16::from_le_bytes(buffer[1..3].try_into().internal_err(msg)?)
                    as u32
                    + 1,
                bytestream_count: u16::from_le_bytes(buffer[3..5].try_into().internal_err(msg)?),
            })
        } else if buffer[0] == 2 {
            let mut buffer = [0_u8; 3];
            reader
                .read_exact(&mut buffer)
                .read_err("Failed to read ignore packet header")?;
            let msg = "Wrong header offsets detected";
            Ok(PacketHeader::Ignored {
                _packet_length: u16::from_le_bytes(buffer[1..3].try_into().internal_err(msg)?)
                    as u32
                    + 1,
            })
        } else {
            Error::invalid("Found unknown packet type when trying to read a packet header")?
        }
    }
}

#[derive(Debug)]
struct PacketFlags {
    pub _compressor_restart: bool,
}

impl PacketFlags {
    pub fn from_byte(value: u8) -> Self {
        Self {
            _compressor_restart: value & 1 != 0,
        }
    }
}

pub fn extract_pointcloud<T: Read + Seek>(
    pc: &PointCloud,
    reader: &mut PagedReader<T>,
) -> Result<Vec<CartesianCoodinate>> {
    reader
        .seek_physical(pc.file_offset)
        .read_err("Cannot seek to compressed vector header")?;
    let section_header = CompressedVectorHeader::from_reader(reader)?;

    reader
        .seek_physical(section_header.data_start_offset)
        .read_err("Cannot seek to packet header")?;

    let mut result = Vec::with_capacity(pc.records as usize);
    while result.len() < pc.records as usize {
        let packet_header = PacketHeader::from_reader(reader)?;
        match packet_header {
            PacketHeader::Index { .. } => todo!(),
            PacketHeader::Ignored { .. } => todo!(),
            PacketHeader::Data {
                bytestream_count, ..
            } => {
                if bytestream_count as usize != pc.prototype.len() {
                    Error::invalid("Bytestream count does not match prototype size")?
                }

                let mut buffer_sizes = Vec::with_capacity(pc.prototype.len());
                for _ in 0..bytestream_count {
                    let mut buf = [0_u8; 2];
                    reader.read_exact(&mut buf).unwrap();
                    let len = u16::from_le_bytes(buf) as usize;
                    buffer_sizes.push(len);
                }

                let mut buffers = Vec::with_capacity(buffer_sizes.len());
                for l in buffer_sizes {
                    let mut buffer = vec![0_u8; l];
                    reader.read_exact(&mut buffer).unwrap();
                    buffers.push(buffer);
                }

                if pc.prototype.len() < 3 {
                    // Prototypes with less than 3 records cannot contain XYZ
                    todo!();
                }

                match (&pc.prototype[0], &pc.prototype[1], &pc.prototype[2]) {
                    (Record::CartesianX(xrt), Record::CartesianY(yrt), Record::CartesianZ(zrt)) => {
                        let x_buffer = BitPack::unpack_double(&buffers[0], xrt)?;
                        let y_buffer = BitPack::unpack_double(&buffers[1], yrt)?;
                        let z_buffer = BitPack::unpack_double(&buffers[2], zrt)?;

                        if x_buffer.len() != y_buffer.len() || y_buffer.len() != z_buffer.len() {
                            Error::invalid(
                                "X, Y and Z buffer in data packet do not have the same size",
                            )?
                        }

                        for i in 0..x_buffer.len() {
                            result.push(CartesianCoodinate {
                                x: x_buffer[i],
                                y: y_buffer[i],
                                z: z_buffer[i],
                            });
                        }
                    }
                    _ => todo!(),
                }
            }
        };

        reader
            .align()
            .read_err("Failed to align on 4-byte offset for next packet")?;
    }

    Ok(result)
}
