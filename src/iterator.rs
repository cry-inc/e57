use crate::bitpack::BitPack;
use crate::comp_vector::CompressedVectorHeader;
use crate::comp_vector::PacketHeader;
use crate::error::Converter;
use crate::paged_reader::PagedReader;
use crate::CartesianCoodinate;
use crate::Color;
use crate::Error;
use crate::Point;
use crate::PointCloud;
use crate::Record;
use crate::Result;
use std::io::{Read, Seek};

pub struct PointCloudIterator<'a, T: Read + Seek> {
    pc: PointCloud,
    reader: &'a mut PagedReader<T>,
    buffer: Vec<Point>,
    buffer_index: usize,
    extracted: u64,
    read: u64,
}

impl<'a, T: Read + Seek> PointCloudIterator<'a, T> {
    fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        reader
            .seek_physical(pc.file_offset)
            .read_err("Cannot seek to compressed vector header")?;
        let section_header = CompressedVectorHeader::from_reader(reader)?;
        reader
            .seek_physical(section_header.data_start_offset)
            .read_err("Cannot seek to packet header")?;
        let pc = pc.clone();
        Ok(PointCloudIterator {
            pc,
            reader,
            buffer: Vec::new(),
            buffer_index: 0,
            extracted: 0,
            read: 0,
        })
    }

    fn advance(&mut self) -> Result<()> {
        self.buffer_index = 0;
        self.buffer.clear();

        if self.extracted >= self.pc.records {
            return Ok(());
        }

        let packet_header = PacketHeader::from_reader(self.reader)?;
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
                if bytestream_count as usize != self.pc.prototype.len() {
                    Error::invalid("Bytestream count does not match prototype size")?
                }

                let mut buffer_sizes = Vec::with_capacity(self.pc.prototype.len());
                for _ in 0..bytestream_count {
                    let mut buf = [0_u8; 2];
                    self.reader.read_exact(&mut buf).unwrap();
                    let len = u16::from_le_bytes(buf) as usize;
                    buffer_sizes.push(len);
                }

                let mut buffers = Vec::with_capacity(buffer_sizes.len());
                for l in buffer_sizes {
                    let mut buffer = vec![0_u8; l];
                    self.reader.read_exact(&mut buffer).unwrap();
                    buffers.push(buffer);
                }

                let mut length = 0;

                let mut x = Vec::new();
                let mut y = Vec::new();
                let mut z = Vec::new();

                let mut red = Vec::new();
                let mut green = Vec::new();
                let mut blue = Vec::new();

                let mut intensity = Vec::new();

                let mut cartesian_invalid = Vec::new();
                let mut spherical_invalid = Vec::new();
                let mut time_invalid = Vec::new();
                let mut intensity_invalid = Vec::new();
                let mut color_invalid = Vec::new();

                let mut handle_length = |len: usize, record: &Record| -> Result<()> {
                    if length == 0 {
                        length = len;
                    }
                    if length != len {
                        Error::invalid(format!(
                            "Other buffers do not have the same size as {record:?}. Found {len} but expected {length}",
                        ))?
                    }
                    Ok(())
                };

                for (i, r) in self.pc.prototype.iter().enumerate() {
                    match r {
                        Record::CartesianX(xrt) => {
                            x = BitPack::unpack_double(&buffers[i], xrt)?;
                            handle_length(x.len(), r)?;
                        }
                        Record::CartesianY(yrt) => {
                            y = BitPack::unpack_double(&buffers[i], yrt)?;
                            handle_length(y.len(), r)?;
                        }
                        Record::CartesianZ(zrt) => {
                            z = BitPack::unpack_double(&buffers[i], zrt)?;
                            handle_length(z.len(), r)?;
                        }
                        Record::ColorRed(rrt) => {
                            red = BitPack::unpack_unit_float(&buffers[i], rrt)?;
                            handle_length(red.len(), r)?;
                        }
                        Record::ColorGreen(grt) => {
                            green = BitPack::unpack_unit_float(&buffers[i], grt)?;
                            handle_length(green.len(), r)?;
                        }
                        Record::ColorBlue(brt) => {
                            blue = BitPack::unpack_unit_float(&buffers[i], brt)?;
                            handle_length(blue.len(), r)?;
                        }
                        Record::Intensity(rt) => {
                            intensity = BitPack::unpack_unit_float(&buffers[i], rt)?;
                            handle_length(intensity.len(), r)?;
                        }
                        Record::CartesianInvalidState(rt) => {
                            cartesian_invalid = BitPack::unpack_u8(&buffers[i], rt)?;
                        }
                        Record::SphericalInvalidState(rt) => {
                            spherical_invalid = BitPack::unpack_u8(&buffers[i], rt)?;
                        }
                        Record::IsTimeStampInvalid(rt) => {
                            time_invalid = BitPack::unpack_u8(&buffers[i], rt)?;
                        }
                        Record::IsIntensityInvalid(rt) => {
                            intensity_invalid = BitPack::unpack_u8(&buffers[i], rt)?;
                        }
                        Record::IsColorInvalid(rt) => {
                            color_invalid = BitPack::unpack_u8(&buffers[i], rt)?;
                        }
                        _ => Error::not_implemented(format!(
                            "Iterator support for record {r:?} is not implemented"
                        ))?,
                    };
                }

                if !x.is_empty() && (y.len() != x.len() || z.len() != x.len()) {
                    Error::invalid("Found incomplete cartesian coordinates: X, Y or Z is missing or incomplete")?
                }
                let has_cartesian = !x.is_empty();

                if !red.is_empty() && (green.len() != red.len() || blue.len() != red.len()) {
                    Error::invalid(
                        "Found incomplete colors: Red, green or blue is missing or incomplete",
                    )?
                }
                let has_color = !red.is_empty();

                for i in 0..length {
                    let mut point = Point::default();
                    if has_cartesian {
                        point.cartesian = Some(CartesianCoodinate {
                            x: x[i],
                            y: y[i],
                            z: z[i],
                        });
                    }
                    if has_color {
                        point.color = Some(Color {
                            red: red[i],
                            green: green[i],
                            blue: blue[i],
                        });
                    }

                    if !intensity.is_empty() {
                        point.intensity = Some(intensity[i]);
                    }

                    if cartesian_invalid.len() >= length {
                        point.cartesian_invalid = Some(cartesian_invalid[i]);
                    }
                    if spherical_invalid.len() >= length {
                        point.spherical_invalid = Some(spherical_invalid[i]);
                    }
                    if time_invalid.len() >= length {
                        point.time_invalid = Some(time_invalid[i]);
                    }
                    if intensity_invalid.len() >= length {
                        point.intensity_invalid = Some(intensity_invalid[i]);
                    }
                    if color_invalid.len() >= length {
                        point.color_invalid = Some(color_invalid[i]);
                    }
                    self.buffer.push(point);
                    self.extracted += 1;
                }
            }
        };

        self.reader
            .align()
            .read_err("Failed to align on 4-byte offset for next packet")?;

        Ok(())
    }
}

impl<'a, T: Read + Seek> Iterator for PointCloudIterator<'a, T> {
    type Item = Result<Point>;
    fn next(&mut self) -> Option<Self::Item> {
        // Check if current buffer is consumed and
        // try advanced to next buffer if required
        if self.buffer_index >= self.buffer.len() {
            if let Err(err) = self.advance() {
                return Some(Err(err));
            }
        }

        // Are there any points left to read?
        if self.buffer_index < self.buffer.len() && self.read < self.pc.records {
            let point = self.buffer[self.buffer_index].clone();
            self.buffer_index += 1;
            self.read += 1;
            Some(Ok(point))
        } else {
            None
        }
    }
}

pub fn pointcloud_iterator<'a, T: Read + Seek>(
    pc: &PointCloud,
    reader: &'a mut PagedReader<T>,
) -> Result<PointCloudIterator<'a, T>> {
    PointCloudIterator::new(pc, reader)
}
