use e57::{
    Blob, CartesianCoordinate, DateTime, E57Reader, E57Writer, Extension, ImageFormat, Point,
    Projection, Quaternion, RawValues, Record, RecordDataType, RecordName, RecordValue, Result,
    SphericalImageProperties, Transform, Translation, VisualReferenceImageProperties,
};
use std::f32::consts::PI;
use std::fs::{remove_file, File};
use std::io::{Cursor, Seek};
use std::path::Path;

#[test]
fn write_read_cycle_points() {
    let path = Path::new("write_read_cycle_points.e57");

    let mut e57 = E57Writer::from_file(path, "guid_file").unwrap();
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
    let mut pc_writer = e57.add_pointcloud("guid_pointcloud", prototype).unwrap();
    for p in points {
        pc_writer.add_point(p).unwrap();
    }
    pc_writer.finalize().unwrap();
    e57.finalize().unwrap();
    drop(e57);

    let mut e57 = E57Reader::from_file(path).unwrap();
    assert_eq!(e57.guid(), "guid_file");
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    for pc in pointclouds {
        assert_eq!(pc.guid.as_deref(), Some("guid_pointcloud"));
        assert_eq!(pc.prototype.len(), 6);
        assert_eq!(pc.records, 2);
        let iter = e57.pointcloud_raw(&pc).unwrap();
        let points: Result<Vec<RawValues>> = iter.collect();
        let points = points.unwrap();
        assert_eq!(points.len(), 2);
    }

    remove_file(path).unwrap();
}

#[test]
fn write_read_cycle_image() {
    let path = Path::new("write_read_cycle_image.e57");
    let mut e57_writer = E57Writer::from_file(path, "guid_file").unwrap();

    let mut img_writer = e57_writer.add_image("guid_image").unwrap();
    img_writer.set_name("name");
    img_writer.set_description("desc");
    img_writer.set_pointcloud_guid("guid_pc");
    img_writer.set_sensor_model("model");
    img_writer.set_sensor_serial("serial");
    img_writer.set_sensor_vendor("vendor");
    img_writer.set_acquisition(DateTime {
        gps_time: 1.1,
        atomic_reference: false,
    });
    img_writer.set_transform(Transform {
        rotation: Quaternion {
            w: 2.2,
            x: 3.3,
            y: 4.4,
            z: 5.5,
        },
        translation: Translation {
            x: 6.6,
            y: 7.7,
            z: 8.8,
        },
    });
    let mut reader = File::open("testdata/castle.jpg").unwrap();
    let props = VisualReferenceImageProperties {
        width: 100,
        height: 100,
    };
    img_writer
        .add_visual_reference(ImageFormat::Jpeg, &mut reader, props, None)
        .unwrap();

    reader.rewind().unwrap();
    let props = SphericalImageProperties {
        width: 100,
        height: 100,
        pixel_width: 3.6,
        pixel_height: 1.8,
    };
    img_writer
        .add_spherical(ImageFormat::Jpeg, &mut reader, props, None)
        .unwrap();
    img_writer.finalize().unwrap();
    e57_writer.finalize().unwrap();
    drop(e57_writer);

    let mut e57 = E57Reader::from_file(path).unwrap();
    assert_eq!(e57.guid(), "guid_file");

    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 0);

    let mut images = e57.images();
    assert_eq!(images.len(), 1);
    let img = images.remove(0);

    assert_eq!(img.guid.as_deref(), Some("guid_image"));
    assert_eq!(img.acquisition.unwrap().gps_time, 1.1);
    assert_eq!(img.name.unwrap(), "name");
    assert_eq!(img.description.unwrap(), "desc");
    assert_eq!(img.pointcloud_guid.unwrap(), "guid_pc");
    assert_eq!(img.sensor_model.unwrap(), "model");
    assert_eq!(img.sensor_serial.unwrap(), "serial");
    assert_eq!(img.sensor_vendor.unwrap(), "vendor");

    let vis_ref = img.visual_reference.unwrap();
    assert_eq!(vis_ref.properties.width, 100);
    assert_eq!(vis_ref.properties.height, 100);
    assert!(matches!(vis_ref.blob.format, ImageFormat::Jpeg));
    assert_eq!(vis_ref.blob.data.offset, 48);
    assert_eq!(vis_ref.blob.data.length, 7722);
    assert!(vis_ref.mask.is_none());

    let rep = match img.projection.unwrap() {
        Projection::Pinhole(_) => None,
        Projection::Spherical(s) => Some(s),
        Projection::Cylindrical(_) => None,
    }
    .unwrap();
    assert_eq!(rep.properties.width, 100);
    assert_eq!(rep.properties.height, 100);
    assert_eq!(rep.properties.pixel_height, 1.8);
    assert_eq!(rep.properties.pixel_width, 3.6);
    assert!(matches!(rep.blob.format, ImageFormat::Jpeg));
    assert_eq!(rep.blob.data.offset, 7816);
    assert_eq!(rep.blob.data.length, 7722);
    assert!(rep.mask.is_none());

    let mut img_bytes = Vec::new();
    let img_length = e57.blob(&rep.blob.data, &mut img_bytes).unwrap();
    assert_eq!(img_length, img_bytes.len() as u64);
    assert_eq!(img_length, 7722);

    let org_image_data = std::fs::read("testdata/castle.jpg").unwrap();
    assert_eq!(org_image_data.len(), img_bytes.len());
    assert_eq!(org_image_data, img_bytes);

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
        let iter = reader.pointcloud_raw(&pc).unwrap();
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
        let iter = reader.pointcloud_raw(pc).unwrap();
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
            offset: 0.0,
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
        let iter = reader.pointcloud_simple(pc).unwrap();
        let points = iter.collect::<Result<Vec<Point>>>().unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(
            points[0].cartesian,
            CartesianCoordinate::Valid {
                x: -1.0,
                y: -1.0,
                z: -1.0
            }
        );
        assert_eq!(
            points[1].cartesian,
            CartesianCoordinate::Valid {
                x: 1.0,
                y: 1.0,
                z: 1.0
            }
        );
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
fn attribute_extensions() {
    let out_path = Path::new("attribute_extensions.e57");

    {
        let mut writer = E57Writer::from_file(out_path, "attribute_extensions_guid").unwrap();
        let ext1 = Extension {
            namespace: "ext1".to_owned(),
            url: "https://www.corp.com/ext1".to_owned(),
        };
        let ext2 = Extension {
            namespace: "ext2".to_owned(),
            url: "http://institute.org/e57/ext2".to_owned(),
        };
        writer.register_extension(ext1.clone()).unwrap();
        writer.register_extension(ext2.clone()).unwrap();

        const INTEGER_TYPE: RecordDataType = RecordDataType::Integer { min: -10, max: 11 };
        const SCALED_INT: RecordDataType = RecordDataType::ScaledInteger {
            min: -1000,
            max: 1000,
            scale: 0.001,
            offset: 0.0,
        };

        let prototype = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
            Record {
                name: RecordName::Unknown {
                    namespace: ext1.namespace.clone(),
                    name: String::from("some_name"),
                },
                data_type: INTEGER_TYPE,
            },
            Record {
                name: RecordName::Unknown {
                    namespace: ext2.namespace.clone(),
                    name: String::from("some_other_name"),
                },
                data_type: SCALED_INT,
            },
        ];
        let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
        pc_writer
            .add_point(vec![
                RecordValue::Single(1.0),
                RecordValue::Single(2.0),
                RecordValue::Single(3.0),
                RecordValue::Integer(-10),
                RecordValue::ScaledInteger(-1000),
            ])
            .unwrap();
        pc_writer.finalize().unwrap();
        writer.finalize().unwrap();
    }

    {
        let mut reader = E57Reader::from_file(out_path).unwrap();

        let extensions = reader.extensions();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0].namespace, "ext1");
        assert_eq!(extensions[0].url, "https://www.corp.com/ext1");
        assert_eq!(extensions[1].namespace, "ext2");
        assert_eq!(extensions[1].url, "http://institute.org/e57/ext2");

        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();

        let proto = &pc.prototype;
        assert_eq!(proto.len(), 5);
        assert!(matches!(
            proto[3],
            Record {
                name: RecordName::Unknown { .. },
                data_type: RecordDataType::Integer { .. }
            }
        ));
        assert!(matches!(
            proto[4],
            Record {
                name: RecordName::Unknown { .. },
                data_type: RecordDataType::ScaledInteger { .. }
            }
        ));

        let iter = reader.pointcloud_raw(pc).unwrap();
        let read_points = iter.collect::<Result<Vec<RawValues>>>().unwrap();
        assert_eq!(read_points.len(), 1);
        let p1 = &read_points[0];
        assert_eq!(p1.len(), 5);

        assert_eq!(p1[0], RecordValue::Single(1.0));
        assert_eq!(p1[1], RecordValue::Single(2.0));
        assert_eq!(p1[2], RecordValue::Single(3.0));
        assert_eq!(p1[3], RecordValue::Integer(-10));
        assert_eq!(p1[4], RecordValue::ScaledInteger(-1000));

        let bounds = pc.cartesian_bounds.as_ref().unwrap();
        assert_eq!(bounds.x_min.unwrap(), 1.0);
        assert_eq!(bounds.y_min.unwrap(), 2.0);
        assert_eq!(bounds.z_min.unwrap(), 3.0);
        assert_eq!(bounds.x_max.unwrap(), 1.0);
        assert_eq!(bounds.y_max.unwrap(), 2.0);
        assert_eq!(bounds.z_max.unwrap(), 3.0);
    }

    remove_file(out_path).unwrap();
}

#[test]
fn unknown_namespace_name_fails() {
    let out_path = Path::new("unknown_namespace.e57");
    let mut writer = E57Writer::from_file(out_path, "unknown_namespace_guid").unwrap();
    let prototype = vec![
        Record::CARTESIAN_X_F64,
        Record::CARTESIAN_Y_F64,
        Record::CARTESIAN_Z_F64,
        Record {
            name: RecordName::Unknown {
                namespace: "ext".to_owned(),
                name: "some_name".to_owned(),
            },
            data_type: RecordDataType::Double {
                min: None,
                max: None,
            },
        },
    ];
    assert!(writer.add_pointcloud("pc_guid", prototype.clone()).is_err());

    let ext = Extension::new("ext", "https://cop.com/ext");
    assert!(writer.register_extension(ext.clone()).is_ok());
    assert!(writer.register_extension(ext.clone()).is_err());
    assert!(writer.add_pointcloud("pc_guid", prototype.clone()).is_ok());

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
            let point = vec![
                RecordValue::Single(incr * i as f32),
                RecordValue::Single(PI),
                RecordValue::Single(1.0),
            ];
            pc_writer.add_point(point).unwrap();
        }

        pc_writer.finalize().unwrap();
        writer.finalize().unwrap();
    }

    {
        let mut reader = E57Reader::from_file(out_path).unwrap();
        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        let iter = reader.pointcloud_raw(pc).unwrap();
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
    let guids = vec![String::from("guid1"), String::from("guid2")];

    {
        let mut e57_writer = E57Writer::from_file(out_path, "file_guid").unwrap();
        e57_writer.set_coordinate_metadata(Some("coord meta".to_owned()));
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
        pc_writer.set_original_guids(Some(guids.clone()));
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
        assert!(creation.atomic_reference);
        assert_eq!(e57_reader.coordinate_metadata(), Some("coord meta"));
        let library_version = e57_reader.library_version().unwrap();
        assert!(library_version.contains("Rust E57 Library"));
        assert!(library_version.contains("github.com/cry-inc/e57"));

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
        assert_eq!(pc.original_guids, Some(guids));
        let start = pc.acquisition_start.unwrap();
        assert_eq!(start.gps_time, 0.0);
        assert!(!start.atomic_reference);
        let end = pc.acquisition_end.unwrap();
        assert_eq!(end.gps_time, 1.23);
        assert!(!start.atomic_reference);
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

#[test]
fn create_empty_e57_file() {
    let out_path = Path::new("empty_e57_file.e57");

    {
        let mut writer = E57Writer::from_file(out_path, "file_guid").unwrap();
        writer.finalize().unwrap();
    }

    {
        let reader = E57Reader::from_file(out_path).unwrap();
        assert_eq!(reader.extensions().len(), 0);
        assert_eq!(reader.pointclouds().len(), 0);
        assert_eq!(reader.images().len(), 0);
    }

    remove_file(out_path).unwrap();
}

#[test]
fn write_read_empty_point_cloud() {
    let path = Path::new("write_read_empty_point_cloud.e57");

    {
        let mut e57_writer = E57Writer::from_file(path, "guid_file").unwrap();
        let prototype = vec![
            Record::CARTESIAN_X_F64,
            Record::CARTESIAN_Y_F64,
            Record::CARTESIAN_Z_F64,
        ];
        let mut pc_writer = e57_writer
            .add_pointcloud("guid_pointcloud", prototype)
            .unwrap();
        pc_writer.finalize().unwrap();
        e57_writer.finalize().unwrap();
    }

    {
        let mut e57 = E57Reader::from_file(path).unwrap();
        assert_eq!(e57.guid(), "guid_file");
        let pointclouds = e57.pointclouds();
        assert_eq!(pointclouds.len(), 1);
        for pc in pointclouds {
            assert_eq!(pc.guid.as_deref(), Some("guid_pointcloud"));
            assert_eq!(pc.prototype.len(), 3);
            assert_eq!(pc.records, 0);
            let iter = e57.pointcloud_raw(&pc).unwrap();
            let points: Result<Vec<RawValues>> = iter.collect();
            let points = points.unwrap();
            assert_eq!(points.len(), 0);
        }
    }

    remove_file(path).unwrap();
}

#[test]
fn write_read_index_bounds() {
    let path = Path::new("write_read_index_bounds.e57");

    let mut e57 = E57Writer::from_file(path, "guid_file").unwrap();
    let prototype = vec![
        Record::CARTESIAN_X_F32,
        Record::CARTESIAN_Y_F32,
        Record::CARTESIAN_Z_F32,
        Record {
            name: RecordName::ColumnIndex,
            data_type: RecordDataType::Integer { min: 0, max: 1023 },
        },
        Record {
            name: RecordName::RowIndex,
            data_type: RecordDataType::Integer { min: 0, max: 1023 },
        },
        Record {
            name: RecordName::ReturnCount,
            data_type: RecordDataType::Integer { min: 0, max: 1023 },
        },
        Record {
            name: RecordName::ReturnIndex,
            data_type: RecordDataType::Integer { min: 0, max: 1023 },
        },
    ];
    let mut pc_writer = e57.add_pointcloud("guid_pointcloud", prototype).unwrap();
    pc_writer
        .add_point(vec![
            RecordValue::Single(1.1),
            RecordValue::Single(2.2),
            RecordValue::Single(3.3),
            RecordValue::Integer(11),
            RecordValue::Integer(12),
            RecordValue::Integer(13),
            RecordValue::Integer(14),
        ])
        .unwrap();
    pc_writer
        .add_point(vec![
            RecordValue::Single(4.4),
            RecordValue::Single(5.5),
            RecordValue::Single(6.6),
            RecordValue::Integer(21),
            RecordValue::Integer(22),
            RecordValue::Integer(23),
            RecordValue::Integer(24),
        ])
        .unwrap();
    pc_writer.finalize().unwrap();
    e57.finalize().unwrap();
    drop(e57);

    let e57 = E57Reader::from_file(path).unwrap();
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    let pc = pointclouds.first().unwrap();
    assert_eq!(pc.prototype.len(), 7);
    let bounds = pc.index_bounds.clone().unwrap();
    assert_eq!(bounds.column_min, Some(11));
    assert_eq!(bounds.column_max, Some(21));
    assert_eq!(bounds.row_min, Some(12));
    assert_eq!(bounds.row_max, Some(22));
    assert_eq!(bounds.return_min, Some(14));
    assert_eq!(bounds.return_max, Some(24));

    remove_file(path).unwrap();
}

#[test]
fn write_read_int_min_max_equal() {
    let path = Path::new("write_read_int_min_max_equal.e57");

    let mut e57 = E57Writer::from_file(path, "guid_file").unwrap();
    let prototype = vec![
        Record::CARTESIAN_X_F32,
        Record::CARTESIAN_Y_F32,
        Record::CARTESIAN_Z_F32,
        Record {
            name: RecordName::ColorRed,
            data_type: RecordDataType::Integer { min: 0, max: 0 },
        },
        Record {
            name: RecordName::ColorGreen,
            data_type: RecordDataType::Integer {
                min: 1111,
                max: 1111,
            },
        },
        Record {
            name: RecordName::ColorBlue,
            data_type: RecordDataType::Integer {
                min: -1111,
                max: -1111,
            },
        },
        Record {
            name: RecordName::Intensity,
            data_type: RecordDataType::ScaledInteger {
                min: 10,
                max: 10,
                scale: 2.1,
                offset: 100.2,
            },
        },
    ];
    let mut pc_writer = e57.add_pointcloud("guid_pointcloud", prototype).unwrap();
    pc_writer
        .add_point(vec![
            RecordValue::Single(1.1),
            RecordValue::Single(2.2),
            RecordValue::Single(3.3),
            RecordValue::Integer(0),
            RecordValue::Integer(1111),
            RecordValue::Integer(-1111),
            RecordValue::ScaledInteger(10),
        ])
        .unwrap();
    pc_writer
        .add_point(vec![
            RecordValue::Single(4.4),
            RecordValue::Single(5.5),
            RecordValue::Single(6.6),
            RecordValue::Integer(0),
            RecordValue::Integer(1111),
            RecordValue::Integer(-1111),
            RecordValue::ScaledInteger(10),
        ])
        .unwrap();
    pc_writer.finalize().unwrap();
    e57.finalize().unwrap();
    drop(e57);

    let mut e57 = E57Reader::from_file(path).unwrap();
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    let pc = pointclouds.first().unwrap();
    assert_eq!(pc.prototype.len(), 7);
    match pc.prototype[3].data_type {
        RecordDataType::Integer { min, max } => {
            assert_eq!(min, 0);
            assert_eq!(max, 0);
        }
        _ => panic!("Unexpected data type"),
    };
    match pc.prototype[4].data_type {
        RecordDataType::Integer { min, max } => {
            assert_eq!(min, 1111);
            assert_eq!(max, 1111);
        }
        _ => panic!("Unexpected data type"),
    };
    match pc.prototype[5].data_type {
        RecordDataType::Integer { min, max } => {
            assert_eq!(min, -1111);
            assert_eq!(max, -1111);
        }
        _ => panic!("Unexpected data type"),
    };
    match pc.prototype[6].data_type {
        RecordDataType::ScaledInteger {
            min,
            max,
            scale,
            offset,
        } => {
            assert_eq!(min, 10);
            assert_eq!(max, 10);
            assert_eq!(scale, 2.1);
            assert_eq!(offset, 100.2);
        }
        _ => panic!("Unexpected data type"),
    };
    let iter = e57.pointcloud_raw(pc).unwrap();
    for p in iter {
        let p = p.unwrap();
        assert_eq!(p.len(), 7);
        assert_eq!(p[3], RecordValue::Integer(0));
        assert_eq!(p[4], RecordValue::Integer(1111));
        assert_eq!(p[5], RecordValue::Integer(-1111));
        assert_eq!(p[6], RecordValue::ScaledInteger(10));
    }

    remove_file(path).unwrap();
}

#[test]
fn extensions_write_read_blobs() {
    let path = Path::new("extensions_write_read_blobs.e57");
    let binary_data = vec![123_u8; 4096];
    let offset;

    {
        let mut e57_writer = E57Writer::from_file(path, "guid_file").unwrap();
        let mut reader = Cursor::new(&binary_data);
        let blob = e57_writer.add_blob(&mut reader).unwrap();
        assert_eq!(blob.length, binary_data.len() as u64);
        assert!(blob.offset > 0);
        offset = blob.offset;
        e57_writer.finalize().unwrap();
    }

    {
        let mut e57 = E57Reader::from_file(path).unwrap();
        assert_eq!(e57.guid(), "guid_file");
        let blob = Blob::new(offset, binary_data.len() as u64);
        let mut writer = Cursor::new(Vec::new());
        let bytes = e57.blob(&blob, &mut writer).unwrap();
        assert_eq!(bytes, blob.length);
        let data = writer.into_inner();
        assert_eq!(data, binary_data);
    }

    remove_file(path).unwrap();
}

#[test]
fn custom_xml_test() {
    let path = Path::new("custom_xml_test.e57");
    let inserted_xml = "<myext:mytag type=\"Structure\"></myext:mytag>";
    let extension = Extension::new("myext", "https://mycompany.com/myext");

    {
        let mut e57_writer = E57Writer::from_file(path, "guid_file").unwrap();
        e57_writer.register_extension(extension).unwrap();
        e57_writer
            .finalize_customized_xml(|xml| {
                assert!(!xml.contains(inserted_xml));
                let from = "</e57Root>";
                let to = format!("{}\n</e57Root>", inserted_xml);
                Ok(xml.replace(from, &to))
            })
            .unwrap();
    }

    {
        let e57 = E57Reader::from_file(path).unwrap();
        assert_eq!(e57.guid(), "guid_file");
        let xml = e57.xml();
        assert!(xml.contains(inserted_xml));
    }

    remove_file(path).unwrap();
}

#[test]
fn writer_bug_regression_partial_bytes() {
    let file = "writer_bug_regression_partial_bytes.e57";
    {
        let mut writer = e57::E57Writer::from_file(file, "file_uuid").unwrap();
        let proto = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
            Record {
                name: RecordName::Intensity,
                data_type: RecordDataType::ScaledInteger {
                    min: 0,
                    max: 2047,
                    scale: 1.0,
                    offset: 0.0,
                },
            },
        ];
        let mut pc_writer = writer.add_pointcloud("pc_guid", proto).unwrap();
        // exactly the the max packet point count of the internal writer
        for _ in 0..4861 {
            let point = vec![
                RecordValue::Single(0.0),
                RecordValue::Single(0.0),
                RecordValue::Single(0.0),
                RecordValue::ScaledInteger(0),
            ];
            pc_writer.add_point(point).unwrap();
        }
        pc_writer.finalize().unwrap();
        writer.finalize().unwrap();
    }
    {
        let mut reader = e57::E57Reader::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        for pc in pcs {
            let iter = reader.pointcloud_raw(&pc).unwrap();
            for (i, point) in iter.enumerate() {
                if let Err(err) = point {
                    panic!("Error reading point {i} at: {err}");
                }
            }
        }
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn writer_bug_regression_invalid_integers() {
    let file = "writer_bug_regression_invalid_integers.e57";
    {
        let mut writer = e57::E57Writer::from_file(file, "file_uuid").unwrap();
        let proto = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
            Record {
                name: RecordName::Intensity,
                data_type: RecordDataType::ScaledInteger {
                    min: 0,
                    max: 2047,
                    scale: 1.0,
                    offset: 0.0,
                },
            },
        ];
        let mut pc_writer = writer.add_pointcloud("pc_guid", proto).unwrap();
        // must be more than the the max packet point count of the internal writer
        for i in 0..5000 {
            let point = vec![
                RecordValue::Single(0.0),
                RecordValue::Single(0.0),
                RecordValue::Single(0.0),
                RecordValue::ScaledInteger(i % 2048),
            ];
            pc_writer.add_point(point).unwrap();
        }
        pc_writer.finalize().unwrap();
        writer.finalize().unwrap();
    }
    {
        let mut reader = e57::E57Reader::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        for pc in pcs {
            let iter = reader.pointcloud_raw(&pc).unwrap();
            for (i, point) in iter.enumerate() {
                match point {
                    Ok(p) => assert_eq!(p[3], RecordValue::ScaledInteger(i as i64 % 2048)),
                    Err(err) => panic!("Error reading point {i} at: {err}"),
                }
            }
        }
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn empty_namespace_name_fails() {
    let out_path = Path::new("empty_namespace_name_fails.e57");

    let mut writer = E57Writer::from_file(out_path, "missing_namespace_name_guid").unwrap();
    assert!(writer
        .register_extension(Extension {
            namespace: String::new(),
            url: String::from("http://example.com")
        })
        .is_err());

    let prototype = vec![
        Record::CARTESIAN_X_F64,
        Record::CARTESIAN_Y_F64,
        Record::CARTESIAN_Z_F64,
        Record {
            name: RecordName::Unknown {
                namespace: String::new(),
                name: "some_name".to_owned(),
            },
            data_type: RecordDataType::Double {
                min: None,
                max: None,
            },
        },
    ];

    assert!(writer.add_pointcloud("pc_guid", prototype).is_err());

    remove_file(out_path).unwrap();
}
