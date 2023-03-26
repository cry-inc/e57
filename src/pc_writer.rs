use crate::cv_section::CompressedVectorSectionHeader;
use crate::error::Converter;
use crate::packet::DataPacketHeader;
use crate::E57Writer;
use crate::Point;
use crate::PointCloud;
use crate::Record;
use crate::RecordType;
use crate::Result;
use std::io::{Read, Seek, Write};

pub struct PointCloudWriter<'a, T: Read + Write + Seek> {
    parent: &'a mut E57Writer<T>,
    guid: String,
    section_offset: u64,
    section_header: CompressedVectorSectionHeader,
    prototype: Vec<Record>,
    point_count: u64,
}

impl<'a, T: Read + Write + Seek> PointCloudWriter<'a, T> {
    pub fn new(parent: &'a mut E57Writer<T>, guid: &str) -> Result<Self> {
        let section_offset = parent.writer.physical_position()?;

        let mut section_header = CompressedVectorSectionHeader::default();
        section_header.data_offset = section_offset + CompressedVectorSectionHeader::SIZE;
        section_header.section_length = CompressedVectorSectionHeader::SIZE;
        section_header.write(&mut parent.writer)?;

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
        ];

        Ok(PointCloudWriter {
            parent,
            guid: guid.to_owned(),
            section_offset,
            section_header,
            prototype,
            point_count: 0,
        })
    }

    pub fn add_points(&mut self, points: &[Point]) -> Result<()> {
        let max_points_per_buffer: usize = 64000 / 3 / 8;
        while self.point_count < points.len() as u64 {
            let mut buffer_x = Vec::new();
            let mut buffer_y = Vec::new();
            let mut buffer_z = Vec::new();
            let packet_points = max_points_per_buffer.min(points.len() - self.point_count as usize);
            for _ in 0..packet_points {
                let p = &points[self.point_count as usize];
                let c = p
                    .cartesian
                    .as_ref()
                    .invalid_err("Missing cartesian coordinates")?;
                buffer_x.extend_from_slice(&c.x.to_le_bytes());
                buffer_y.extend_from_slice(&c.y.to_le_bytes());
                buffer_z.extend_from_slice(&c.z.to_le_bytes());
                self.point_count += 1;
            }

            let mut packet_length = DataPacketHeader::SIZE + 3 * 2 + packet_points as u64 * 8 * 3;
            if packet_length % 4 != 0 {
                let missing = 4 - (packet_length % 4);
                packet_length += missing;
            }
            self.section_header.section_length += packet_length;

            DataPacketHeader {
                comp_restart_flag: false,
                packet_length,
                bytestream_count: 3,
            }
            .write(&mut self.parent.writer)?;

            let x_buffer_size = (buffer_x.len() as u16).to_le_bytes();
            self.parent
                .writer
                .write_all(&x_buffer_size)
                .write_err("Cannot write data packet buffer size for X")?;
            let y_buffer_size = (buffer_y.len() as u16).to_le_bytes();
            self.parent
                .writer
                .write_all(&y_buffer_size)
                .write_err("Cannot write data packet buffer size for Y")?;
            let z_buffer_size = (buffer_z.len() as u16).to_le_bytes();
            self.parent
                .writer
                .write_all(&z_buffer_size)
                .write_err("Cannot write data packet buffer size for Z")?;

            self.parent
                .writer
                .write_all(&buffer_x)
                .write_err("Cannot write data for X")?;
            self.parent
                .writer
                .write_all(&buffer_y)
                .write_err("Cannot write data for Y")?;
            self.parent
                .writer
                .write_all(&buffer_z)
                .write_err("Cannot write data for Z")?;

            self.parent.writer.align().write_err(
                "Failed to align writer on next 4-byte offset after writing data packet",
            )?;
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> Result<()> {
        // We need to write the section header again with the final length
        // which was previously unknown and is now available.
        let end_offset = self
            .parent
            .writer
            .physical_position()
            .write_err("Failed to get section end offset")?;
        self.parent
            .writer
            .physical_seek(self.section_offset)
            .write_err("Failed to seek to section start for final update")?;
        self.section_header.write(&mut self.parent.writer)?;
        self.parent
            .writer
            .physical_seek(end_offset)
            .write_err("Failed to seek behind finalized section")?;

        self.parent.pointclouds.push(PointCloud {
            guid: self.guid.clone(),
            records: self.point_count,
            file_offset: self.section_offset,
            prototype: self.prototype.clone(),
            ..Default::default()
        });

        Ok(())
    }
}
