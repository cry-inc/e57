use crate::error::Converter;
use crate::paged_writer::PagedWriter;
use crate::pc_writer::PointCloudWriter;
use crate::root::{serialize_root, Root};
use crate::{Header, PointCloud, Record, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::Path;

/// Main interface for writing E57 files.
pub struct E57Writer<T: Read + Write + Seek> {
    pub(crate) writer: PagedWriter<T>,
    pub(crate) pointclouds: Vec<PointCloud>,
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

    /// Creates a new writer for adding a new point cloud to the E57 file.
    pub fn add_pointcloud(
        &mut self,
        guid: &str,
        prototype: Vec<Record>,
    ) -> Result<PointCloudWriter<T>> {
        PointCloudWriter::new(&mut self.writer, &mut self.pointclouds, guid, prototype)
    }

    /// Needs to be called after adding all point clouds and images.
    ///
    /// This will generate and write the XML metadata to finalize and complete the E57 file.
    /// Without calling this method before dropping the E57 file will be incomplete and invalid!
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
    use crate::{E57Reader, Point, RawPoint, RecordDataType, RecordName, RecordValue};
    use std::fs::{remove_file, File};
    use std::path::Path;

    #[test]
    fn write_read_cycle() {
        let path = Path::new("write_read_cycle.e57");
        let mut e57_writer = E57Writer::from_file(path).unwrap();

        let mut p1 = RawPoint::new();
        p1.insert(RecordName::CartesianX, crate::RecordValue::Double(1.1));
        p1.insert(RecordName::CartesianY, crate::RecordValue::Double(2.2));
        p1.insert(RecordName::CartesianZ, crate::RecordValue::Double(3.3));
        p1.insert(RecordName::ColorRed, crate::RecordValue::Integer(255));
        p1.insert(RecordName::ColorGreen, crate::RecordValue::Integer(0));
        p1.insert(RecordName::ColorBlue, crate::RecordValue::Integer(0));
        let mut p2 = RawPoint::new();
        p2.insert(RecordName::CartesianX, crate::RecordValue::Double(4.4));
        p2.insert(RecordName::CartesianY, crate::RecordValue::Double(5.5));
        p2.insert(RecordName::CartesianZ, crate::RecordValue::Double(6.6));
        p2.insert(RecordName::ColorRed, crate::RecordValue::Integer(0));
        p2.insert(RecordName::ColorGreen, crate::RecordValue::Integer(0));
        p2.insert(RecordName::ColorBlue, crate::RecordValue::Integer(255));

        let mut points = Vec::new();
        points.push(p1);
        points.push(p2);

        let prototype = vec![
            Record {
                name: RecordName::CartesianX,
                data_type: RecordDataType::Double {
                    min: None,
                    max: None,
                },
            },
            Record {
                name: RecordName::CartesianY,
                data_type: RecordDataType::Double {
                    min: None,
                    max: None,
                },
            },
            Record {
                name: RecordName::CartesianZ,
                data_type: RecordDataType::Double {
                    min: None,
                    max: None,
                },
            },
            Record {
                name: RecordName::ColorRed,
                data_type: RecordDataType::Integer { min: 0, max: 255 },
            },
            Record {
                name: RecordName::ColorGreen,
                data_type: RecordDataType::Integer { min: 0, max: 255 },
            },
            Record {
                name: RecordName::ColorBlue,
                data_type: RecordDataType::Integer { min: 0, max: 255 },
            },
        ];

        let mut pc_writer = e57_writer
            .add_pointcloud("guid_pointcloud", prototype)
            .unwrap();
        for p in points {
            pc_writer.add_point(p).unwrap();
        }
        pc_writer.finalize().unwrap();
        e57_writer.finalize("guid_file").unwrap();
        drop(e57_writer);

        {
            let file = File::open(path).unwrap();
            let xml = E57Reader::raw_xml(file).unwrap();
            assert!(xml.len() > 0);
            //std::fs::write("test.xml", xml).unwrap();
        }

        let mut e57 = E57Reader::from_file(path).unwrap();
        assert_eq!(e57.guid(), "guid_file");
        let pointclouds = e57.pointclouds();
        assert_eq!(pointclouds.len(), 1);
        for pc in pointclouds {
            assert_eq!(pc.guid, "guid_pointcloud");
            assert_eq!(pc.prototype.len(), 6);
            assert_eq!(pc.records, 2);
            //println!("PC: {pc:#?}");

            let iter = e57.pointcloud(&pc).unwrap();
            let points: Result<Vec<Point>> = iter.collect();
            let points = points.unwrap();
            assert_eq!(points.len(), 2);
            //println!("Points: {points:#?}");
        }

        remove_file(path).unwrap();
    }

    #[test]
    fn copy_tiny() {
        let in_path = Path::new("testdata/tinyCartesianFloatRgb.e57");
        let out_path = Path::new("tiny_copy.e57");

        let original_points = {
            let mut reader = E57Reader::from_file(in_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            iter.collect::<Result<Vec<Point>>>().unwrap()
        };

        {
            let mut writer = E57Writer::from_file(out_path).unwrap();
            let prototype = vec![
                Record {
                    name: RecordName::CartesianX,
                    data_type: RecordDataType::Double {
                        min: None,
                        max: None,
                    },
                },
                Record {
                    name: RecordName::CartesianY,
                    data_type: RecordDataType::Double {
                        min: None,
                        max: None,
                    },
                },
                Record {
                    name: RecordName::CartesianZ,
                    data_type: RecordDataType::Double {
                        min: None,
                        max: None,
                    },
                },
                Record {
                    name: RecordName::CartesianInvalidState,
                    data_type: RecordDataType::Integer { min: 0, max: 2 },
                },
                Record {
                    name: RecordName::ColorRed,
                    data_type: RecordDataType::Integer { min: 0, max: 255 },
                },
                Record {
                    name: RecordName::ColorGreen,
                    data_type: RecordDataType::Integer { min: 0, max: 255 },
                },
                Record {
                    name: RecordName::ColorBlue,
                    data_type: RecordDataType::Integer { min: 0, max: 255 },
                },
            ];
            let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
            for p in &original_points {
                let mut rp = RawPoint::new();
                let xyz = p.cartesian.as_ref().unwrap();
                rp.insert(RecordName::CartesianX, RecordValue::Double(xyz.x));
                rp.insert(RecordName::CartesianY, RecordValue::Double(xyz.y));
                rp.insert(RecordName::CartesianZ, RecordValue::Double(xyz.z));
                rp.insert(RecordName::CartesianInvalidState, RecordValue::Integer(0));
                let rgb = p.color.as_ref().unwrap();
                rp.insert(
                    RecordName::ColorRed,
                    RecordValue::Integer((rgb.red * 255.0) as i64),
                );
                rp.insert(
                    RecordName::ColorGreen,
                    RecordValue::Integer((rgb.green * 255.0) as i64),
                );
                rp.insert(
                    RecordName::ColorBlue,
                    RecordValue::Integer((rgb.blue * 255.0) as i64),
                );
                pc_writer.add_point(rp).unwrap();
            }
            pc_writer.finalize().unwrap();
            writer.finalize("file_guid").unwrap();
        }

        let duplicated_points = {
            let mut reader = E57Reader::from_file(out_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            iter.collect::<Result<Vec<Point>>>().unwrap()
        };

        assert_eq!(original_points.len(), duplicated_points.len());

        remove_file(out_path).unwrap();
    }

    #[test]
    fn scaled_integers_test() {
        let out_path = Path::new("scaled_ints.e57");

        {
            let mut writer = E57Writer::from_file(out_path).unwrap();
            let prototype = vec![
                Record {
                    name: RecordName::CartesianX,
                    data_type: RecordDataType::ScaledInteger {
                        min: -1000,
                        max: 1000,
                        scale: 0.001,
                    },
                },
                Record {
                    name: RecordName::CartesianY,
                    data_type: RecordDataType::ScaledInteger {
                        min: -1000,
                        max: 1000,
                        scale: 0.001,
                    },
                },
                Record {
                    name: RecordName::CartesianZ,
                    data_type: RecordDataType::ScaledInteger {
                        min: -1000,
                        max: 1000,
                        scale: 0.001,
                    },
                },
            ];
            let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();

            let mut rp1 = RawPoint::new();
            rp1.insert(RecordName::CartesianX, RecordValue::ScaledInteger(-1000));
            rp1.insert(RecordName::CartesianY, RecordValue::ScaledInteger(-1000));
            rp1.insert(RecordName::CartesianZ, RecordValue::ScaledInteger(-1000));
            pc_writer.add_point(rp1).unwrap();

            let mut rp2 = RawPoint::new();
            rp2.insert(RecordName::CartesianX, RecordValue::ScaledInteger(1000));
            rp2.insert(RecordName::CartesianY, RecordValue::ScaledInteger(1000));
            rp2.insert(RecordName::CartesianZ, RecordValue::ScaledInteger(1000));
            pc_writer.add_point(rp2).unwrap();

            pc_writer.finalize().unwrap();
            writer.finalize("file_guid").unwrap();
        }

        let read_points = {
            let mut reader = E57Reader::from_file(out_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            iter.collect::<Result<Vec<Point>>>().unwrap()
        };

        assert_eq!(read_points.len(), 2);
        assert_eq!(read_points[0].cartesian.as_ref().unwrap().x, -1.0);
        assert_eq!(read_points[0].cartesian.as_ref().unwrap().y, -1.0);
        assert_eq!(read_points[0].cartesian.as_ref().unwrap().z, -1.0);
        assert_eq!(read_points[1].cartesian.as_ref().unwrap().x, 1.0);
        assert_eq!(read_points[1].cartesian.as_ref().unwrap().y, 1.0);
        assert_eq!(read_points[1].cartesian.as_ref().unwrap().z, 1.0);

        remove_file(out_path).unwrap();
    }
}
