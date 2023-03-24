use crate::error::Converter;
use crate::paged_writer::PagedWriter;
use crate::root::{serialize_root, Root};
use crate::{Header, Point, PointCloud, Record, RecordType, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::Path;

/// Main interface for writing E57 files.
pub struct E57Writer<T: Read + Write + Seek> {
    writer: PagedWriter<T>,
    pointclouds: Vec<PointCloud>,
}

impl<T: Write + Read + Seek> E57Writer<T> {
    /// Creates a new E57 generator from a writer that must also implement Read and Seek.
    ///
    /// Keep in mind that File::create() will not work as input because it only opens the file for writing!
    pub fn new(writer: T) -> Result<Self> {
        // Set up paged writer abstraction for CRC
        let mut writer = PagedWriter::new(writer)?;

        // Write placeholder header that will be replaced later
        let header = Header::default();
        header.write(&mut writer)?;

        Ok(Self {
            writer,
            pointclouds: Vec::new(),
        })
    }

    pub fn add_xyz_pointcloud(&mut self, guid: &str, points: &[Point]) -> Result<()> {
        let offset = self.writer.physical_position()?;
        let mut comp_vec_header = [0_u8; 32];
        comp_vec_header[0] = 1;
        comp_vec_header[16..24].copy_from_slice(&(offset + 32).to_le_bytes());
        self.writer
            .write_all(&comp_vec_header)
            .write_err("Failed to write temporary compressed vector section header")?;

        let mut point = 0;
        let max_points_per_buffer: usize = 64000 / 3 / 8;
        while point < points.len() {
            let mut buffer_x = Vec::new();
            let mut buffer_y = Vec::new();
            let mut buffer_z = Vec::new();
            let packet_points = max_points_per_buffer.min(points.len() - point);
            for _ in 0..packet_points {
                let p = &points[point];
                let c = p
                    .cartesian
                    .as_ref()
                    .invalid_err("Missing cartesian coordinates")?;
                buffer_x.extend_from_slice(&c.x.to_le_bytes());
                buffer_y.extend_from_slice(&c.y.to_le_bytes());
                buffer_z.extend_from_slice(&c.z.to_le_bytes());
                point += 1;
            }

            let mut data_packet_header = [0_u8; 6];
            data_packet_header[0] = 1;
            let data_packet_length =
                (6 + 3 * 2 + buffer_x.len() + buffer_y.len() + buffer_z.len() - 1) as u16;
            data_packet_header[2..4].copy_from_slice(&data_packet_length.to_le_bytes());
            data_packet_header[4..6].copy_from_slice(&3_u16.to_le_bytes());
            self.writer
                .write_all(&data_packet_header)
                .write_err("Failed to write temporary data packet header")?;

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

            self.writer
                .write_all(&buffer_x)
                .write_err("Cannot write data for X")?;
            self.writer
                .write_all(&buffer_y)
                .write_err("Cannot write data for Y")?;
            self.writer
                .write_all(&buffer_z)
                .write_err("Cannot write data for Z")?;

            // todo skip to 4 byte alignment?
        }

        let pointcloud = PointCloud {
            guid: guid.to_owned(),
            records: points.len() as u64,
            file_offset: offset,
            prototype: vec![
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
            ],
            ..Default::default()
        };
        self.pointclouds.push(pointcloud);

        Ok(())
    }

    pub fn finalize(&mut self, guid: &str) -> Result<()> {
        // Serialize XML data and write
        let root = Root {
            guid: guid.to_owned(),
            ..Default::default()
        };
        let xml = serialize_root(&root, &self.pointclouds)?;
        let xml_bytes = xml.as_bytes();
        let xml_length = xml_bytes.len();
        let xml_offset = self.writer.physical_position()?;
        self.writer
            .write_all(xml_bytes)
            .write_err("Failed to write XML data")?;
        let phys_length = self.writer.physical_size()?;

        // Add missing values in header at start of the the file
        let header = Header {
            phys_xml_offset: xml_offset,
            xml_length: xml_length as u64,
            phys_length,
            ..Default::default()
        };
        self.writer.physical_seek(0)?;
        header.write(&mut self.writer)?;
        self.writer
            .flush()
            .write_err("Failed to flush writer at the end")
    }
}

impl E57Writer<File> {
    /// Creates an E57 writer instance from a Path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(true)
            .open(path)
            .read_err("Unable to create file for writing, reading and seeking")?;
        Self::new(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CartesianCoordinate, E57Reader, Point};
    use std::fs::File;
    use std::path::Path;

    #[test]
    #[ignore]
    fn write_read_cycle() {
        let path = Path::new("test.e57");
        let mut e57_writer = E57Writer::from_file(path).unwrap();
        let points = [
            Point {
                cartesian: Some(CartesianCoordinate {
                    x: 1.1,
                    y: 2.2,
                    z: 3.3,
                }),
                ..Default::default()
            },
            Point {
                cartesian: Some(CartesianCoordinate {
                    x: 4.4,
                    y: 5.5,
                    z: 6.6,
                }),
                ..Default::default()
            },
        ];
        e57_writer
            .add_xyz_pointcloud("guid_pointcloud", &points)
            .unwrap();
        e57_writer.finalize("guid_file").unwrap();
        drop(e57_writer);

        {
            let file = File::open(path).unwrap();
            let xml = E57Reader::raw_xml(file).unwrap();
            std::fs::write("test.xml", xml).unwrap();
        }

        let mut e57 = E57Reader::from_file(path).unwrap();
        let pointclouds = e57.pointclouds();
        for pc in pointclouds {
            println!("PC: {pc:#?}");
            let iter = e57.pointcloud(&pc).unwrap();
            let points: Result<Vec<Point>> = iter.collect();
            let points = points.unwrap();
            println!("Points: {points:#?}");
        }
    }
}
