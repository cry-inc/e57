use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::DataPacketHeader;
use crate::paged_writer::PagedWriter;
use crate::Point;
use crate::PointCloud;
use crate::Record;
use crate::RecordType;
use crate::Result;
use std::collections::VecDeque;
use std::io::{Read, Seek, Write};

/// Creates a new point cloud by consuming points and writing them into an E57 file.
pub struct PointCloudWriter<'a, T: Read + Write + Seek> {
    writer: &'a mut PagedWriter<T>,
    pointclouds: &'a mut Vec<PointCloud>,
    guid: String,
    section_offset: u64,
    section_header: CompressedVectorSectionHeader,
    prototype: Vec<Record>,
    point_count: u64,
    buffer: VecDeque<Point>,
    max_points_per_packet: usize,
}

impl<'a, T: Read + Write + Seek> PointCloudWriter<'a, T> {
    pub(crate) fn new(
        writer: &'a mut PagedWriter<T>,
        pointclouds: &'a mut Vec<PointCloud>,
        guid: &str,
    ) -> Result<Self> {
        let section_offset = writer.physical_position()?;

        let mut section_header = CompressedVectorSectionHeader::default();
        section_header.data_offset = section_offset + CompressedVectorSectionHeader::SIZE;
        section_header.section_length = CompressedVectorSectionHeader::SIZE;
        section_header.write(writer)?;

        let prototype = vec![
            Record::CartesianX(RecordType::Double {
                min: None,
                max: None,
            }),
            Record::CartesianY(RecordType::Double {
                min: None,
                max: None,
            }),
            Record::CartesianZ(RecordType::Double {
                min: None,
                max: None,
            }),
            Record::ColorRed(RecordType::Integer { min: 0, max: 255 }),
            Record::ColorGreen(RecordType::Integer { min: 0, max: 255 }),
            Record::ColorBlue(RecordType::Integer { min: 0, max: 255 }),
        ];

        Ok(PointCloudWriter {
            writer,
            pointclouds,
            guid: guid.to_owned(),
            section_offset,
            section_header,
            prototype,
            point_count: 0,
            buffer: VecDeque::new(),
            max_points_per_packet: 64000 / 27,
        })
    }

    fn write_buffer_to_disk(&mut self) -> Result<()> {
        let packet_points = self.max_points_per_packet.min(self.buffer.len());
        if packet_points > 0 {
            let mut buffer_x = Vec::new();
            let mut buffer_y = Vec::new();
            let mut buffer_z = Vec::new();
            let mut buffer_r = Vec::new();
            let mut buffer_g = Vec::new();
            let mut buffer_b = Vec::new();
            for _ in 0..packet_points {
                let p = self
                    .buffer
                    .pop_front()
                    .internal_err("Failed to get next point for writing")?;
                let coords = p
                    .cartesian
                    .as_ref()
                    .invalid_err("Missing cartesian coordinates")?;
                buffer_x.extend_from_slice(&coords.x.to_le_bytes());
                buffer_y.extend_from_slice(&coords.y.to_le_bytes());
                buffer_z.extend_from_slice(&coords.z.to_le_bytes());
                let colors = p.color.as_ref().invalid_err("Missing color values")?;
                buffer_r.push((&colors.red * 255.0) as u8);
                buffer_g.push((&colors.green * 255.0) as u8);
                buffer_b.push((&colors.blue * 255.0) as u8);
            }

            // Calculate packet length for header
            let mut packet_length = DataPacketHeader::SIZE
                + self.prototype.len() as u64 * 2
                + packet_points as u64 * 27;
            if packet_length % 4 != 0 {
                let missing = 4 - (packet_length % 4);
                packet_length += missing;
            }
            self.section_header.section_length += packet_length;

            // Write header
            DataPacketHeader {
                comp_restart_flag: false,
                packet_length,
                bytestream_count: self.prototype.len() as u16,
            }
            .write(&mut self.writer)?;

            // Write bytestream sizes as u16 values
            let x_buffer_size = (buffer_x.len() as u16).to_le_bytes();
            self.writer
                .write_all(&x_buffer_size)
                .write_err("Cannot write data packet buffer size for X")?;
            let y_buffer_size = (buffer_y.len() as u16).to_le_bytes();
            self.writer
                .write_all(&y_buffer_size)
                .write_err("Cannot write data packet buffer size for Y")?;
            let z_buffer_size = (buffer_z.len() as u16).to_le_bytes();
            self.writer
                .write_all(&z_buffer_size)
                .write_err("Cannot write data packet buffer size for Z")?;
            let r_buffer_size = (buffer_r.len() as u16).to_le_bytes();
            self.writer
                .write_all(&r_buffer_size)
                .write_err("Cannot write data packet buffer size for red")?;
            let g_buffer_size = (buffer_g.len() as u16).to_le_bytes();
            self.writer
                .write_all(&g_buffer_size)
                .write_err("Cannot write data packet buffer size for green")?;
            let b_buffer_size = (buffer_b.len() as u16).to_le_bytes();
            self.writer
                .write_all(&b_buffer_size)
                .write_err("Cannot write data packet buffer size for blue")?;

            // Write actual bytestream buffers with data
            self.writer
                .write_all(&buffer_x)
                .write_err("Cannot write data for X")?;
            self.writer
                .write_all(&buffer_y)
                .write_err("Cannot write data for Y")?;
            self.writer
                .write_all(&buffer_z)
                .write_err("Cannot write data for Z")?;
            self.writer
                .write_all(&buffer_r)
                .write_err("Cannot write data for red")?;
            self.writer
                .write_all(&buffer_g)
                .write_err("Cannot write data for green")?;
            self.writer
                .write_all(&buffer_b)
                .write_err("Cannot write data for blue")?;

            self.writer.align().write_err(
                "Failed to align writer on next 4-byte offset after writing data packet",
            )?;
        }
        Ok(())
    }

    /// Adds a new point to the point cloud.
    pub fn add_point(&mut self, point: Point) -> Result<()> {
        self.buffer.push_back(point);
        self.point_count += 1;
        if self.buffer.len() >= self.max_points_per_packet {
            self.write_buffer_to_disk()?;
        }
        Ok(())
    }

    /// Called after all points have been added to finalize the creation of the new point cloud.
    pub fn finalize(&mut self) -> Result<()> {
        // Flush remaining points from buffer
        while !self.buffer.is_empty() {
            self.write_buffer_to_disk()?;
        }

        // We need to write the section header again with the final length
        // which was previously unknown and is now available.
        let end_offset = self
            .writer
            .physical_position()
            .write_err("Failed to get section end offset")?;
        self.writer
            .physical_seek(self.section_offset)
            .write_err("Failed to seek to section start for final update")?;
        self.section_header.write(&mut self.writer)?;
        self.writer
            .physical_seek(end_offset)
            .write_err("Failed to seek behind finalized section")?;

        self.pointclouds.push(PointCloud {
            guid: self.guid.clone(),
            records: self.point_count,
            file_offset: self.section_offset,
            prototype: self.prototype.clone(),
            ..Default::default()
        });

        Ok(())
    }
}
