use crate::bitpack::BitPack;
use crate::bs_read::ByteStreamReadBuffer;
use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::PacketHeader;
use crate::paged_reader::PagedReader;
use crate::point::Return;
use crate::CartesianCoordinate;
use crate::Color;
use crate::Error;
use crate::Point;
use crate::PointCloud;
use crate::RecordName;
use crate::Result;
use crate::SphericalCoordinate;
use std::collections::VecDeque;
use std::io::{Read, Seek};

/// Iterate over all points of an existing point cloud to read it.
pub struct PointCloudReader<'a, T: Read + Seek> {
    pc: PointCloud,
    reader: &'a mut PagedReader<T>,
    byte_streams: Vec<ByteStreamReadBuffer>,
    read: u64,
    queue_x: VecDeque<f64>,
    queue_y: VecDeque<f64>,
    queue_z: VecDeque<f64>,
    queue_range: VecDeque<f64>,
    queue_azimuth: VecDeque<f64>,
    queue_elevation: VecDeque<f64>,
    queue_time: VecDeque<f64>,
    queue_red: VecDeque<f32>,
    queue_green: VecDeque<f32>,
    queue_blue: VecDeque<f32>,
    queue_intensity: VecDeque<f32>,
    queue_row: VecDeque<i64>,
    queue_column: VecDeque<i64>,
    queue_return_index: VecDeque<i64>,
    queue_return_count: VecDeque<i64>,
    queue_cartesian_invalid: VecDeque<u8>,
    queue_spherical_invalid: VecDeque<u8>,
    queue_time_invalid: VecDeque<u8>,
    queue_intensity_invalid: VecDeque<u8>,
    queue_color_invalid: VecDeque<u8>,
}

impl<'a, T: Read + Seek> PointCloudReader<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        reader
            .seek_physical(pc.file_offset)
            .read_err("Cannot seek to compressed vector header")?;
        let section_header = CompressedVectorSectionHeader::read(reader)?;
        reader
            .seek_physical(section_header.data_offset)
            .read_err("Cannot seek to packet header")?;
        let byte_streams = vec![ByteStreamReadBuffer::new(); pc.prototype.len()];
        let pc = pc.clone();

        Ok(PointCloudReader {
            pc,
            reader,
            read: 0,
            byte_streams,
            queue_x: VecDeque::new(),
            queue_y: VecDeque::new(),
            queue_z: VecDeque::new(),
            queue_range: VecDeque::new(),
            queue_azimuth: VecDeque::new(),
            queue_elevation: VecDeque::new(),
            queue_time: VecDeque::new(),
            queue_red: VecDeque::new(),
            queue_green: VecDeque::new(),
            queue_blue: VecDeque::new(),
            queue_intensity: VecDeque::new(),
            queue_row: VecDeque::new(),
            queue_column: VecDeque::new(),
            queue_return_index: VecDeque::new(),
            queue_return_count: VecDeque::new(),
            queue_cartesian_invalid: VecDeque::new(),
            queue_spherical_invalid: VecDeque::new(),
            queue_time_invalid: VecDeque::new(),
            queue_intensity_invalid: VecDeque::new(),
            queue_color_invalid: VecDeque::new(),
        })
    }

    fn available_in_queue(&self) -> usize {
        let mut available: Option<usize> = None;
        for r in &self.pc.prototype {
            let len = match r.name {
                RecordName::CartesianX => self.queue_x.len(),
                RecordName::CartesianY => self.queue_y.len(),
                RecordName::CartesianZ => self.queue_z.len(),
                RecordName::CartesianInvalidState => self.queue_cartesian_invalid.len(),
                RecordName::SphericalRange => self.queue_range.len(),
                RecordName::SphericalAzimuth => self.queue_azimuth.len(),
                RecordName::SphericalElevation => self.queue_elevation.len(),
                RecordName::SphericalInvalidState => self.queue_spherical_invalid.len(),
                RecordName::Intensity => self.queue_intensity.len(),
                RecordName::IsIntensityInvalid => self.queue_intensity_invalid.len(),
                RecordName::ColorRed => self.queue_red.len(),
                RecordName::ColorGreen => self.queue_green.len(),
                RecordName::ColorBlue => self.queue_blue.len(),
                RecordName::IsColorInvalid => self.queue_color_invalid.len(),
                RecordName::RowIndex => self.queue_row.len(),
                RecordName::ColumnIndex => self.queue_column.len(),
                RecordName::ReturnCount => self.queue_return_count.len(),
                RecordName::ReturnIndex => self.queue_return_index.len(),
                RecordName::TimeStamp => self.queue_time.len(),
                RecordName::IsTimeStampInvalid => self.queue_time_invalid.len(),
            };
            match available {
                Some(old_len) => {
                    if len < old_len {
                        available = Some(len);
                    }
                }
                None => {
                    available = Some(len);
                }
            }
        }
        available.unwrap_or(0)
    }

    fn pop_queue_point(&mut self) -> Point {
        let mut point = Point::default();
        for r in &self.pc.prototype {
            match r.name {
                RecordName::CartesianX => {
                    point.cartesian = Some(CartesianCoordinate {
                        x: self.queue_x.pop_front().unwrap(),
                        y: self.queue_y.pop_front().unwrap(),
                        z: self.queue_z.pop_front().unwrap(),
                    })
                }
                RecordName::CartesianY => {}
                RecordName::CartesianZ => {}
                RecordName::CartesianInvalidState => {
                    point.cartesian_invalid =
                        Some(self.queue_cartesian_invalid.pop_front().unwrap())
                }
                RecordName::SphericalRange => {
                    point.spherical = Some(SphericalCoordinate {
                        range: self.queue_range.pop_front().unwrap(),
                        azimuth: self.queue_azimuth.pop_front().unwrap(),
                        elevation: self.queue_elevation.pop_front().unwrap(),
                    })
                }
                RecordName::SphericalAzimuth => {}
                RecordName::SphericalElevation => {}
                RecordName::SphericalInvalidState => {
                    point.spherical_invalid =
                        Some(self.queue_spherical_invalid.pop_front().unwrap())
                }
                RecordName::Intensity => {
                    point.intensity = Some(self.queue_intensity.pop_front().unwrap())
                }
                RecordName::IsIntensityInvalid => {
                    point.intensity_invalid =
                        Some(self.queue_intensity_invalid.pop_front().unwrap())
                }
                RecordName::ColorRed => {
                    point.color = Some(Color {
                        red: self.queue_red.pop_front().unwrap(),
                        green: self.queue_green.pop_front().unwrap(),
                        blue: self.queue_blue.pop_front().unwrap(),
                    })
                }
                RecordName::ColorGreen => {}
                RecordName::ColorBlue => {}
                RecordName::IsColorInvalid => {
                    point.color_invalid = Some(self.queue_color_invalid.pop_front().unwrap())
                }
                RecordName::RowIndex => point.row = Some(self.queue_row.pop_front().unwrap()),
                RecordName::ColumnIndex => {
                    point.column = Some(self.queue_column.pop_front().unwrap())
                }
                RecordName::ReturnCount => {
                    point.ret = Some(Return {
                        count: self.queue_return_count.pop_front().unwrap(),
                        index: self.queue_return_index.pop_front().unwrap(),
                    })
                }
                RecordName::ReturnIndex => {}
                RecordName::TimeStamp => point.time = Some(self.queue_time.pop_front().unwrap()),
                RecordName::IsTimeStampInvalid => {
                    point.time_invalid = Some(self.queue_time_invalid.pop_front().unwrap())
                }
            };
        }
        point
    }

    fn advance(&mut self) -> Result<()> {
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

                let mut buffer_sizes = Vec::with_capacity(self.byte_streams.len());
                for _ in 0..header.bytestream_count {
                    let mut buf = [0_u8; 2];
                    self.reader
                        .read_exact(&mut buf)
                        .read_err("Failed to read data packet buffer sizes")?;
                    let len = u16::from_le_bytes(buf) as usize;
                    buffer_sizes.push(len);
                }

                for (i, bs) in buffer_sizes.iter().enumerate() {
                    let mut buffer = vec![0_u8; *bs];
                    self.reader
                        .read_exact(&mut buffer)
                        .read_err("Failed to read data packet buffers")?;
                    self.byte_streams[i].append(buffer);
                }

                for (i, r) in self.pc.prototype.iter().enumerate() {
                    let rt = &r.data_type;
                    match r.name {
                        RecordName::CartesianX => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_x);
                        }
                        RecordName::CartesianY => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_y);
                        }
                        RecordName::CartesianZ => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_z);
                        }
                        RecordName::SphericalRange => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_range);
                        }
                        RecordName::SphericalAzimuth => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_azimuth);
                        }
                        RecordName::SphericalElevation => {
                            let v = BitPack::unpack_double(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_elevation);
                        }
                        RecordName::ColorRed => {
                            let v = BitPack::unpack_unit_float(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_red);
                        }
                        RecordName::ColorGreen => {
                            let v = BitPack::unpack_unit_float(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_green);
                        }
                        RecordName::ColorBlue => {
                            let v = BitPack::unpack_unit_float(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_blue);
                        }
                        RecordName::Intensity => {
                            let v = BitPack::unpack_unit_float(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_intensity);
                        }
                        RecordName::CartesianInvalidState => {
                            let v = BitPack::unpack_u8(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_cartesian_invalid);
                        }
                        RecordName::SphericalInvalidState => {
                            let v = BitPack::unpack_u8(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_spherical_invalid);
                        }
                        RecordName::IsTimeStampInvalid => {
                            let v = BitPack::unpack_u8(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_time_invalid);
                        }
                        RecordName::IsIntensityInvalid => {
                            let v = BitPack::unpack_u8(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_intensity_invalid);
                        }
                        RecordName::IsColorInvalid => {
                            let v = BitPack::unpack_u8(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_color_invalid);
                        }
                        RecordName::RowIndex => {
                            let v = BitPack::unpack_i64(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_row);
                        }
                        RecordName::ColumnIndex => {
                            let v = BitPack::unpack_i64(&mut self.byte_streams[i], rt)?;
                            append_vec_to_queue(&v, &mut self.queue_column);
                        }
                        _ => Error::not_implemented(format!(
                            "Iterator support for record {r:?} is not implemented"
                        ))?,
                    };
                }

                if !self.queue_x.is_empty()
                    && (self.queue_y.len() != self.queue_x.len()
                        || self.queue_z.len() != self.queue_x.len())
                {
                    Error::invalid("Found incomplete cartesian coordinates: X, Y or Z is missing or incomplete")?
                }

                if !self.queue_red.is_empty()
                    && (self.queue_green.len() != self.queue_red.len()
                        || self.queue_blue.len() != self.queue_red.len())
                {
                    Error::invalid(
                        "Found incomplete colors: Red, green or blue is missing or incomplete",
                    )?
                }
            }
        };

        self.reader
            .align()
            .read_err("Failed to align reader on next 4-byte offset after reading packet")?;

        Ok(())
    }
}

impl<'a, T: Read + Seek> Iterator for PointCloudReader<'a, T> {
    /// Each iterator item is a result for an extracted point.
    type Item = Result<Point>;

    /// Returns the next available point or None if the end was reached.
    fn next(&mut self) -> Option<Self::Item> {
        // Already read all points?
        if self.read >= self.pc.records {
            return None;
        }

        // Refill property queues if required
        if self.available_in_queue() < 1 {
            if let Err(err) = self.advance() {
                return Some(Err(err));
            }
        }

        // Try to read next point from properties queues
        if self.available_in_queue() > 0 {
            let point = self.pop_queue_point();
            self.read += 1;
            Some(Ok(point))
        } else {
            None
        }
    }
}

fn append_vec_to_queue<T: Copy>(v: &Vec<T>, q: &mut VecDeque<T>) {
    for e in v {
        q.push_back(*e)
    }
}
