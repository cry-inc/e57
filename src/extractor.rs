use crate::bitpack::BitPack;
use crate::comp_vector::CompressedVectorHeader;
use crate::comp_vector::PacketHeader;
use crate::error::Converter;
use crate::paged_reader::PagedReader;
use crate::CartesianCoodinate;
use crate::Error;
use crate::PointCloud;
use crate::Record;
use crate::Result;
use std::io::{Read, Seek};

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
            PacketHeader::Index { .. } => {
                Error::not_implemented("Index packets are not yet supported")?
            }
            PacketHeader::Ignored { .. } => {
                Error::not_implemented("Ignored packets are not yet supported")?
            }
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
                    Error::not_implemented(
                        "This library does currently not support prototypes with less than 3 records",
                    )?
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
                    _ => Error::not_implemented(
                        "This file contains an combination of protoypes that is currently not supported",
                    )?
                }
            }
        };

        reader
            .align()
            .read_err("Failed to align on 4-byte offset for next packet")?;
    }

    // In some cases the bytestreams seem to contain more points that expected with invalid data at the end
    result.truncate(pc.records as usize);

    Ok(result)
}
