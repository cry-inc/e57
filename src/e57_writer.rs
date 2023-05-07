use crate::error::Converter;
use crate::paged_writer::PagedWriter;
use crate::pc_writer::PointCloudWriter;
use crate::root::{serialize_root, Root};
use crate::{DateTime, Header, PointCloud, Record, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::Path;

/// Main interface for creating and writing E57 files.
pub struct E57Writer<T: Read + Write + Seek> {
    pub(crate) writer: PagedWriter<T>,
    pub(crate) pointclouds: Vec<PointCloud>,
    root: Root,
}

impl<T: Write + Read + Seek> E57Writer<T> {
    /// Creates a new E57 generator from a writer that must also implement Read and Seek.
    ///
    /// Keep in mind that File::create() will not work as input because it only opens the file for writing!
    pub fn new(writer: T, guid: &str) -> Result<Self> {
        // Set up paged writer abstraction for CRC
        let mut writer = PagedWriter::new(writer)?;

        // Write placeholder header that will be replaced later
        let header = Header::default();
        header.write(&mut writer)?;

        let version = env!("CARGO_PKG_VERSION");
        let root = Root {
            guid: guid.to_owned(),
            library_version: Some(format!("Rust E57 Library v{version}")),
            ..Default::default()
        };

        Ok(Self {
            writer,
            pointclouds: Vec::new(),
            root,
        })
    }

    /// Set optional coordinate metadata string (empty by default).
    pub fn set_coordinate_metadata(&mut self, value: Option<String>) {
        self.root.coordinate_metadata = value;
    }

    /// Set optional creation date time (empty by default).
    pub fn set_creation(&mut self, value: Option<DateTime>) {
        self.root.creation = value;
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
    pub fn finalize(&mut self) -> Result<()> {
        let xml = serialize_root(&self.root, &self.pointclouds)?;
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
    pub fn from_file(path: impl AsRef<Path>, guid: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .truncate(true)
            .open(path)
            .read_err("Unable to create file for writing, reading and seeking")?;
        Self::new(file, guid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        E57Reader, Point, Quaternion, RawValues, RecordDataType, RecordName, RecordValue,
        Transform, Translation,
    };
    use std::f32::consts::PI;
    use std::fs::{remove_file, File};
    use std::path::Path;

    #[test]
    fn write_read_cycle() {
        let path = Path::new("write_read_cycle.e57");
        let mut e57_writer = E57Writer::from_file(path, "guid_file").unwrap();

        let prototype = vec![
            Record::CARTESIAN_X_F64,
            Record::CARTESIAN_Y_F64,
            Record::CARTESIAN_Z_F64,
            Record::COLOR_RED_U8,
            Record::COLOR_GREEN_U8,
            Record::COLOR_BLUE_U8,
        ];

        let points = vec![
            vec![
                RecordValue::Double(1.1),
                RecordValue::Double(2.2),
                RecordValue::Double(3.3),
                RecordValue::Integer(255),
                RecordValue::Integer(0),
                RecordValue::Integer(0),
            ],
            vec![
                RecordValue::Double(4.4),
                RecordValue::Double(5.5),
                RecordValue::Double(6.6),
                RecordValue::Integer(0),
                RecordValue::Integer(0),
                RecordValue::Integer(255),
            ],
        ];

        let mut pc_writer = e57_writer
            .add_pointcloud("guid_pointcloud", prototype)
            .unwrap();
        for p in points {
            pc_writer.add_point(p).unwrap();
        }
        pc_writer.finalize().unwrap();
        e57_writer.finalize().unwrap();
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
            let points: Result<Vec<RawValues>> = iter.collect();
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

        let (org_points, org_pc) = {
            let mut reader = E57Reader::from_file(in_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap().clone();
            let iter = reader.pointcloud(&pc).unwrap();
            (iter.collect::<Result<Vec<RawValues>>>().unwrap(), pc)
        };

        {
            let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();
            let mut prototype = org_pc.prototype.clone();
            prototype.push(Record {
                name: RecordName::CartesianInvalidState,
                data_type: RecordDataType::Integer { min: 0, max: 2 },
            });
            let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
            for p in &org_points {
                let mut p = p.clone();
                p.push(RecordValue::Integer(0));
                pc_writer.add_point(p).unwrap();
            }
            pc_writer.finalize().unwrap();
            writer.finalize().unwrap();
        }

        let duplicated_points = {
            let mut reader = E57Reader::from_file(out_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            iter.collect::<Result<Vec<RawValues>>>().unwrap()
        };

        assert_eq!(org_points.len(), duplicated_points.len());

        remove_file(out_path).unwrap();
    }

    #[test]
    fn scaled_integers() {
        let out_path = Path::new("scaled_integers.e57");

        {
            let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();
            const SCALED_INT: RecordDataType = RecordDataType::ScaledInteger {
                min: -1000,
                max: 1000,
                scale: 0.001,
            };
            let prototype = vec![
                Record {
                    name: RecordName::CartesianX,
                    data_type: SCALED_INT,
                },
                Record {
                    name: RecordName::CartesianY,
                    data_type: SCALED_INT,
                },
                Record {
                    name: RecordName::CartesianZ,
                    data_type: SCALED_INT,
                },
            ];
            let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
            pc_writer
                .add_point(vec![
                    RecordValue::ScaledInteger(-1000),
                    RecordValue::ScaledInteger(-1000),
                    RecordValue::ScaledInteger(-1000),
                ])
                .unwrap();
            pc_writer
                .add_point(vec![
                    RecordValue::ScaledInteger(1000),
                    RecordValue::ScaledInteger(1000),
                    RecordValue::ScaledInteger(1000),
                ])
                .unwrap();
            pc_writer.finalize().unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = E57Reader::from_file(out_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            let read_points = iter.collect::<Result<Vec<RawValues>>>().unwrap();
            assert_eq!(read_points.len(), 2);
            let p1 = Point::from_values(read_points[0].clone(), &pc.prototype).unwrap();
            assert_eq!(p1.cartesian.as_ref().unwrap().x, -1.0);
            assert_eq!(p1.cartesian.as_ref().unwrap().y, -1.0);
            assert_eq!(p1.cartesian.as_ref().unwrap().z, -1.0);
            let p2 = Point::from_values(read_points[1].clone(), &pc.prototype).unwrap();
            assert_eq!(p2.cartesian.as_ref().unwrap().x, 1.0);
            assert_eq!(p2.cartesian.as_ref().unwrap().y, 1.0);
            assert_eq!(p2.cartesian.as_ref().unwrap().z, 1.0);
            let bounds = pc.cartesian_bounds.as_ref().unwrap();
            assert_eq!(bounds.x_min.unwrap(), -1.0);
            assert_eq!(bounds.y_min.unwrap(), -1.0);
            assert_eq!(bounds.z_min.unwrap(), -1.0);
            assert_eq!(bounds.x_max.unwrap(), 1.0);
            assert_eq!(bounds.y_max.unwrap(), 1.0);
            assert_eq!(bounds.z_max.unwrap(), 1.0);
        }

        remove_file(out_path).unwrap();
    }

    #[test]
    fn spherical_coordinates() {
        let out_path = Path::new("spherical_coordinates.e57");

        {
            let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();
            let prototype = vec![
                Record {
                    name: RecordName::SphericalAzimuth,
                    data_type: RecordDataType::F32,
                },
                Record {
                    name: RecordName::SphericalElevation,
                    data_type: RecordDataType::F32,
                },
                Record {
                    name: RecordName::SphericalRange,
                    data_type: RecordDataType::F32,
                },
            ];
            let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();

            let incr = (2.0 * PI) / 99.0;
            for i in 0..100 {
                let mut values = RawValues::with_capacity(3);
                values.push(RecordValue::Single(incr * i as f32));
                values.push(RecordValue::Single(PI));
                values.push(RecordValue::Single(1.0));
                pc_writer.add_point(values).unwrap();
            }

            pc_writer.finalize().unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = E57Reader::from_file(out_path).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let iter = reader.pointcloud(pc).unwrap();
            let read_points = iter.collect::<Result<Vec<RawValues>>>().unwrap();
            assert_eq!(read_points.len(), 100);
            let bounds = pc.spherical_bounds.as_ref().unwrap();
            assert_eq!(bounds.azimuth_start.unwrap(), 0.0);
            assert_eq!(bounds.azimuth_end.unwrap(), 2.0 * PI as f64);
            assert_eq!(bounds.elevation_min.unwrap(), PI as f64);
            assert_eq!(bounds.elevation_max.unwrap(), PI as f64);
            assert_eq!(bounds.range_min.unwrap(), 1.0);
            assert_eq!(bounds.range_max.unwrap(), 1.0);
        }

        remove_file(out_path).unwrap();
    }

    #[test]
    fn invalid_prototype() {
        let out_path = Path::new("invalid_prototype.e57");
        let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();

        let prototype = vec![Record::CARTESIAN_X_F32];
        writer.add_pointcloud("pc_guid", prototype).err().unwrap();

        let prototype = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
            Record::COLOR_BLUE_U8,
        ];
        writer.add_pointcloud("pc_guid1", prototype).err().unwrap();

        let prototype = vec![Record {
            name: RecordName::SphericalAzimuth,
            data_type: RecordDataType::F64,
        }];
        writer.add_pointcloud("pc_guid2", prototype).err().unwrap();

        remove_file(out_path).unwrap();
    }

    #[test]
    fn invalid_points() {
        let out_path = Path::new("invalid_points.e57");
        let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();
        let prototype = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
        ];
        let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
        pc_writer.add_point(vec![]).err().unwrap();
        pc_writer
            .add_point(vec![RecordValue::Single(1.0)])
            .err()
            .unwrap();
        pc_writer
            .add_point(vec![
                RecordValue::Single(1.0),
                RecordValue::Single(1.0),
                RecordValue::Double(1.0),
            ])
            .err()
            .unwrap();

        remove_file(out_path).unwrap();
    }

    #[test]
    fn write_read_meta_data() {
        let out_path = Path::new("metadata.e57");

        {
            let mut e57_writer = E57Writer::from_file(out_path, "file_guid").unwrap();
            e57_writer.set_creation(Some(DateTime {
                gps_time: 12.34,
                atomic_reference: true,
            }));
            let prototype = vec![
                Record::CARTESIAN_X_F32,
                Record::CARTESIAN_Y_F32,
                Record::CARTESIAN_Z_F32,
            ];
            let mut pc_writer = e57_writer.add_pointcloud("pc_guid", prototype).unwrap();
            pc_writer
                .add_point(vec![
                    RecordValue::Single(1.0),
                    RecordValue::Single(1.0),
                    RecordValue::Single(1.0),
                ])
                .unwrap();
            pc_writer.set_name(Some(String::from("name")));
            pc_writer.set_description(Some(String::from("desc")));
            pc_writer.set_sensor_vendor(Some(String::from("vendor")));
            pc_writer.set_sensor_model(Some(String::from("model")));
            pc_writer.set_sensor_serial(Some(String::from("serial")));
            pc_writer.set_sensor_hw_version(Some(String::from("hw")));
            pc_writer.set_sensor_fw_version(Some(String::from("fw")));
            pc_writer.set_sensor_sw_version(Some(String::from("sw")));
            pc_writer.set_acquisition_start(Some(DateTime {
                gps_time: 0.00,
                atomic_reference: false,
            }));
            pc_writer.set_acquisition_end(Some(DateTime {
                gps_time: 1.23,
                atomic_reference: false,
            }));
            pc_writer.set_temperature(Some(23.0));
            pc_writer.set_humidity(Some(66.6));
            pc_writer.set_atmospheric_pressure(Some(1337.0));
            pc_writer.set_transform(Some(Transform {
                rotation: Quaternion {
                    w: 1.1,
                    x: 2.2,
                    y: 3.3,
                    z: 4.4,
                },
                translation: Translation {
                    x: 5.5,
                    y: 6.6,
                    z: 7.7,
                },
            }));
            pc_writer.finalize().unwrap();
            e57_writer.finalize().unwrap();
        }

        {
            let e57_reader = E57Reader::from_file(out_path).unwrap();
            let creation = e57_reader.creation().unwrap();
            assert_eq!(creation.gps_time, 12.34);
            assert_eq!(creation.atomic_reference, true);

            let pcs = e57_reader.pointclouds();
            let pc = pcs[0].clone();
            assert_eq!(pc.name, Some(String::from("name")));
            assert_eq!(pc.description, Some(String::from("desc")));
            assert_eq!(pc.sensor_vendor, Some(String::from("vendor")));
            assert_eq!(pc.sensor_model, Some(String::from("model")));
            assert_eq!(pc.sensor_serial, Some(String::from("serial")));
            assert_eq!(pc.sensor_hw_version, Some(String::from("hw")));
            assert_eq!(pc.sensor_fw_version, Some(String::from("fw")));
            assert_eq!(pc.sensor_sw_version, Some(String::from("sw")));
            let start = pc.acquisition_start.unwrap();
            assert_eq!(start.gps_time, 0.0);
            assert_eq!(start.atomic_reference, false);
            let end = pc.acquisition_end.unwrap();
            assert_eq!(end.gps_time, 1.23);
            assert_eq!(end.atomic_reference, false);
            assert_eq!(pc.temperature, Some(23.0));
            assert_eq!(pc.humidity, Some(66.6));
            assert_eq!(pc.atmospheric_pressure, Some(1337.0));
            let transform = pc.transform.unwrap();
            assert_eq!(transform.rotation.w, 1.1);
            assert_eq!(transform.rotation.x, 2.2);
            assert_eq!(transform.rotation.y, 3.3);
            assert_eq!(transform.rotation.z, 4.4);
            assert_eq!(transform.translation.x, 5.5);
            assert_eq!(transform.translation.y, 6.6);
            assert_eq!(transform.translation.z, 7.7);
        }

        remove_file(out_path).unwrap();
    }
}
