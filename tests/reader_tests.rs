use e57::{
    CartesianCoordinate, E57Reader, ImageFormat, Point, Projection, RawValues, Record,
    RecordDataType, RecordName, RecordValue, Result, SphericalCoordinate,
};
use std::fs::File;

#[test]
fn header() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();

    assert_eq!(&header.signature, b"ASTM-E57");
    assert_eq!(header.major, 1);
    assert_eq!(header.minor, 0);
    assert_eq!(header.page_size, 1024);
    assert_eq!(header.phys_length, 743424);
    assert_eq!(header.phys_xml_offset, 740736);
    assert_eq!(header.xml_length, 2172);
}

#[test]
fn validate_crc() {
    let file = File::open("testdata/bunnyDouble.e57").unwrap();
    assert_eq!(E57Reader::validate_crc(file).unwrap(), 1024);

    let file = File::open("testdata/corrupt_crc.e57").unwrap();
    assert!(E57Reader::validate_crc(file).is_err());
}

#[test]
fn raw_xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();

    let reader = File::open("testdata/bunnyDouble.e57").unwrap();
    let xml = E57Reader::raw_xml(reader).unwrap();

    assert_eq!(xml.len(), 2172);
    assert_eq!(xml.len(), header.xml_length as usize);
}

#[test]
fn xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();
    let xml = reader.xml();
    let xml_len = xml.as_bytes().len();

    assert_eq!(xml_len, 2172);
    assert_eq!(xml_len, header.xml_length as usize);
}

#[test]
fn format_name() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let format = reader.format_name();
    assert_eq!(format, "ASTM E57 3D Imaging Data File");
}

#[test]
fn guid() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let guid = reader.guid();
    assert_eq!(guid, "{19AA90ED-145E-4B3B-922C-80BC00648844}");
}

#[test]
fn creation() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let creation = reader.creation().unwrap();
    assert_eq!(creation.gps_time, 987369380.8049808);
    assert_eq!(creation.atomic_reference, false);
}

#[test]
fn empty_extensions() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let extensions = reader.extensions();
    assert_eq!(extensions.len(), 0);
}

#[test]
fn coord_metadata() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let metadata = reader.coordinate_metadata();
    assert_eq!(metadata, Some(""));
}

#[test]
fn pointclouds() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let pcs = reader.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert_eq!(
        pc.guid.as_deref(),
        Some("{9CA24C38-C93E-40E8-A366-F49977C7E3EB}")
    );
    assert_eq!(pc.name.as_deref(), Some("bunny"));
    assert_eq!(pc.file_offset, 48);
    assert_eq!(pc.records, 30571);
    assert_eq!(pc.prototype.len(), 4);
    assert!(matches!(pc.prototype[0].name, RecordName::CartesianX,));
    assert!(matches!(pc.prototype[1].name, RecordName::CartesianY,));
    assert!(matches!(pc.prototype[2].name, RecordName::CartesianZ,));
    assert!(matches!(
        pc.prototype[3].name,
        RecordName::CartesianInvalidState,
    ));

    let reader = E57Reader::from_file("testdata/tinyCartesianFloatRgb.e57").unwrap();
    let pcs = reader.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert_eq!(
        pc.guid.as_deref(),
        Some("{49aa8f8b-618f-423e-a632-f9a58ad79e40}")
    );
    assert_eq!(pc.name.as_deref(), Some("exp2.fls.subsampled"));
    assert_eq!(pc.file_offset, 48);
    assert_eq!(pc.records, 2090);
    assert_eq!(pc.prototype.len(), 6);
    assert!(matches!(pc.prototype[0].name, RecordName::CartesianX,));
    assert!(matches!(pc.prototype[1].name, RecordName::CartesianY,));
    assert!(matches!(pc.prototype[2].name, RecordName::CartesianZ,));
    assert!(matches!(pc.prototype[3].name, RecordName::ColorRed,));
    assert!(matches!(pc.prototype[4].name, RecordName::ColorGreen,));
    assert!(matches!(pc.prototype[5].name, RecordName::ColorBlue,));
}

#[test]
fn bunny_point_count() {
    let files = [
        "testdata/bunnyDouble.e57",
        "testdata/bunnyFloat.e57",
        "testdata/bunnyInt32.e57",
        "testdata/bunnyInt24.e57",
        "testdata/bunnyInt21.e57",
        "testdata/bunnyInt19.e57",
    ];
    for file in files {
        let mut reader = E57Reader::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        assert_eq!(pc.records, 30571);
        let points: Result<Vec<Point>> = reader.pointcloud_simple(pc).unwrap().collect();
        assert_eq!(points.unwrap().len(), 30571);
    }
}

#[test]
fn cartesian_bounds() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let bounds = pc.cartesian_bounds.as_ref().unwrap();
    assert_eq!(bounds.x_min, Some(-9.779529571533203));
    assert_eq!(bounds.x_max, Some(-6.774238109588623));
    assert_eq!(bounds.y_min, Some(4.513879299163818));
    assert_eq!(bounds.y_max, Some(7.51546049118042));
    assert_eq!(bounds.z_min, Some(295.5246887207031));
    assert_eq!(bounds.z_max, Some(298.53216552734375));
}

#[test]
fn color_limits() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let limits = pc.color_limits.as_ref().unwrap();
    assert_eq!(limits.red_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.red_max, Some(RecordValue::Integer(255)));
    assert_eq!(limits.green_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.green_max, Some(RecordValue::Integer(255)));
    assert_eq!(limits.blue_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.blue_max, Some(RecordValue::Integer(255)));
}

#[test]
fn raw_iterator() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    assert_eq!(pc.records, 2090);
    let mut counter = 0;
    for p in reader.pointcloud_raw(pc).unwrap() {
        let p = p.unwrap();
        assert_eq!(p.len(), 6);
        assert!(matches!(p[0], RecordValue::Single(..)));
        assert!(matches!(p[1], RecordValue::Single(..)));
        assert!(matches!(p[2], RecordValue::Single(..)));
        assert!(matches!(p[3], RecordValue::Integer(..)));
        assert!(matches!(p[4], RecordValue::Integer(..)));
        assert!(matches!(p[5], RecordValue::Integer(..)));
        counter += 1;
    }
    assert_eq!(counter, pc.records);
}

#[test]
fn simple_iterator() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    assert_eq!(pc.records, 2090);
    let mut counter = 0;
    for p in reader.pointcloud_simple(pc).unwrap() {
        let p = p.unwrap();
        assert!(matches!(p.cartesian, CartesianCoordinate::Valid { .. }));
        assert!(matches!(p.color, Some(..)));
        counter += 1;
    }
    assert_eq!(counter, pc.records);
}

#[test]
fn iterator_size_hint() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let mut iter = reader.pointcloud_simple(pc).unwrap();

    // Hint at the beginning returns all points
    let hint = iter.size_hint();
    assert_eq!(hint, (2090, Some(2090)));

    // Hint is correctly updated after we consumed one point
    iter.next().unwrap().unwrap();
    let hint = iter.size_hint();
    assert_eq!(hint, (2089, Some(2089)));

    // Reading everything returns the predicted point count
    let points: Result<Vec<Point>> = iter.collect();
    assert_eq!(points.unwrap().len(), 2089);
}

#[test]
fn empty_e57_file() {
    let file = "testdata/empty.e57";
    let reader = E57Reader::from_file(file).unwrap();

    assert_eq!(reader.guid(), "{976E9187-A110-48D1-D58E-5DBF07B1630E}");
    assert_eq!(
        reader.library_version().as_deref(),
        Some("E57Format-3.0.0-AMD64_64-vc1937")
    );
    assert!(reader.coordinate_metadata().is_none());
    assert!(reader.creation().is_none());

    assert_eq!(reader.extensions().len(), 0);
    assert_eq!(reader.pointclouds().len(), 0);
    assert_eq!(reader.images().len(), 0);
}

#[test]
fn with_extension() {
    let file = "testdata/tiny_pc_with_extension.e57";
    let mut reader = E57Reader::from_file(file).unwrap();

    let extensions = reader.extensions();
    assert_eq!(extensions.len(), 1);
    let ext = &extensions[0];
    assert_eq!(ext.namespace, "nor");
    assert_eq!(ext.url, "http://www.libe57.org/E57_EXT_surface_normals.txt");
    assert_eq!(
        reader.library_version().as_deref(),
        Some("E57Format-3.2.0-AMD64_64-vc1940")
    );

    let pointclouds = reader.pointclouds();
    assert_eq!(pointclouds.len(), 1);

    let pointcloud = &pointclouds[0];
    assert_eq!(pointcloud.prototype.len(), 6);
    assert_eq!(
        pointcloud.prototype[3].name,
        RecordName::Unknown {
            namespace: String::from("nor"),
            name: String::from("normalX")
        }
    );
    assert_eq!(
        pointcloud.prototype[4].name,
        RecordName::Unknown {
            namespace: String::from("nor"),
            name: String::from("normalY")
        }
    );
    assert_eq!(
        pointcloud.prototype[5].name,
        RecordName::Unknown {
            namespace: String::from("nor"),
            name: String::from("normalZ")
        }
    );

    assert_eq!(pointcloud.records, 1);
    let iter = reader.pointcloud_raw(pointcloud).unwrap();
    for res in iter {
        let values = res.unwrap();
        assert_eq!(values.len(), 6);
        assert_eq!(values[3], RecordValue::Single(1.0));
        assert_eq!(values[4], RecordValue::Single(0.0));
        assert_eq!(values[5], RecordValue::Single(0.0));
    }
}

#[test]
fn original_guids() {
    let file = "testdata/original_guids.e57";
    let reader = E57Reader::from_file(file).unwrap();
    let pointclouds = reader.pointclouds();
    let pc = pointclouds.first().unwrap();
    let guids = pc.original_guids.as_ref().unwrap();
    assert_eq!(guids.len(), 3);
    assert_eq!(guids[0], "guid1");
    assert_eq!(guids[1], "guid2");
    assert_eq!(guids[2], "guid3");
}

#[test]
fn spherical_coordinates() {
    let file = "testdata/tiny_spherical.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pointclouds = reader.pointclouds();
    let pc = pointclouds.first().unwrap();
    let proto = &pc.prototype;
    assert!(matches!(
        proto[0],
        Record {
            name: RecordName::SphericalRange,
            ..
        }
    ));
    assert!(matches!(
        proto[1],
        Record {
            name: RecordName::SphericalAzimuth,
            ..
        }
    ));
    assert!(matches!(
        proto[2],
        Record {
            name: RecordName::SphericalElevation,
            ..
        }
    ));
    assert!(matches!(
        pc.prototype[3],
        Record {
            name: RecordName::SphericalInvalidState,
            ..
        }
    ));
    let iter = reader.pointcloud_simple(pc).unwrap();
    let mut points = Vec::new();
    for (i, p) in iter.enumerate() {
        let p = p.unwrap();

        // Odd points are direction only
        if i % 2 == 0 {
            assert!(matches!(p.spherical, SphericalCoordinate::Valid { .. }));
        } else {
            assert!(matches!(p.spherical, SphericalCoordinate::Direction { .. }));
        }
        points.push(p);
    }
    assert_eq!(points.len(), 360);
    assert_eq!(points.len(), pc.records as usize);
    assert_eq!(
        points[0].spherical,
        SphericalCoordinate::Valid {
            range: 1.0,
            azimuth: 0.0,
            elevation: 0.0
        }
    );
    let angle = 359.0 * (3.14 / 360.0);
    assert_eq!(
        points[359].spherical,
        SphericalCoordinate::Direction {
            azimuth: angle,
            elevation: angle
        }
    );
}

#[test]
fn read_images() {
    let file = "testdata/tiny_pc_and_images.e57";
    let mut reader = E57Reader::from_file(file).unwrap();

    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let pc_guid = pc.guid.as_deref().unwrap();

    let images = reader.images();
    assert_eq!(images.len(), 4);

    {
        let img = &images[0];
        assert_eq!(
            img.guid.as_deref(),
            Some("{5E36E0E0-D577-478D-8A45-3CA5D4809B0F}")
        );
        assert_eq!(img.name.as_deref(), Some("visual"));
        assert!(img.projection.is_none());
        let vis_ref = img.visual_reference.as_ref().unwrap();
        assert_eq!(vis_ref.blob.data.offset, 116);
        assert_eq!(vis_ref.blob.data.length, 7722);
        assert!(matches!(vis_ref.blob.format, ImageFormat::Jpeg));
        assert_eq!(vis_ref.properties.width, 100);
        assert_eq!(vis_ref.properties.height, 100);
        let mut blob_dump = Vec::new();
        let size = reader.blob(&vis_ref.blob.data, &mut blob_dump).unwrap();
        assert_eq!(size, vis_ref.blob.data.length);
        assert_eq!(blob_dump.len(), size as usize);
    }

    {
        let img = &images[1];
        assert_eq!(
            img.guid.as_deref(),
            Some("{D868A2FD-16AD-40FD-7CE4-AFC4F4CB05AB}")
        );
        assert_eq!(img.name.as_deref(), Some("spherical"));
        assert_eq!(img.description.as_deref(), Some("desc"));
        assert_eq!(img.sensor_vendor.as_deref(), Some("vendor"));
        assert_eq!(img.sensor_model.as_deref(), Some("sensor"));
        assert_eq!(img.sensor_serial.as_deref(), Some("serial"));
        assert_eq!(img.pointcloud_guid.as_deref(), Some(pc_guid));
        let transform = img.transform.as_ref().unwrap();
        assert_eq!(transform.rotation.w, 0.5);
        assert_eq!(transform.rotation.x, 1.0);
        assert_eq!(transform.rotation.y, 0.0);
        assert_eq!(transform.rotation.z, 0.0);
        assert_eq!(transform.translation.x, 1.0);
        assert_eq!(transform.translation.y, 2.0);
        assert_eq!(transform.translation.z, 3.0);
        assert!(img.visual_reference.is_none());
        let projection = img.projection.as_ref().unwrap();
        assert!(matches!(projection, Projection::Spherical(..)));
        let si = if let Projection::Spherical(value) = projection {
            value
        } else {
            panic!("Unexpected projection")
        };
        assert_eq!(si.properties.height, 100);
        assert_eq!(si.properties.width, 100);
        assert_eq!(si.properties.pixel_width, 0.0314);
        assert_eq!(si.properties.pixel_height, 0.0314);
        assert_eq!(si.blob.data.offset, 7884);
        assert_eq!(si.blob.data.length, 1073);
        assert!(matches!(si.blob.format, ImageFormat::Png));
        assert_eq!(si.properties.width, 100);
        assert_eq!(si.properties.height, 100);
        let mut blob_dump = Vec::new();
        let size = reader.blob(&si.blob.data, &mut blob_dump).unwrap();
        assert_eq!(size, si.blob.data.length);
        assert_eq!(blob_dump.len(), size as usize);
    }

    {
        let img = &images[2];
        assert_eq!(
            img.guid.as_deref(),
            Some("{3CEABB5C-E41A-49A7-05A5-83BB039D4F14}")
        );
        assert_eq!(img.name.as_deref(), Some("pinhole"));
        let projection = img.projection.as_ref().unwrap();
        assert!(matches!(projection, Projection::Pinhole(..)));
        let pi = if let Projection::Pinhole(value) = projection {
            value
        } else {
            panic!("Unexpected projection")
        };
        assert_eq!(pi.properties.height, 100);
        assert_eq!(pi.properties.width, 100);
        assert_eq!(pi.properties.pixel_width, 0.044);
        assert_eq!(pi.properties.pixel_height, 0.033);
        assert_eq!(pi.properties.focal_length, 123.0);
        assert_eq!(pi.properties.principal_x, 23.0);
        assert_eq!(pi.properties.principal_y, 42.0);
        assert_eq!(pi.blob.data.offset, 8980);
        assert_eq!(pi.blob.data.length, 1073);
        assert!(matches!(pi.blob.format, ImageFormat::Png));
        assert_eq!(pi.properties.width, 100);
        assert_eq!(pi.properties.height, 100);
        let mut blob_dump = Vec::new();
        let size = reader.blob(&pi.blob.data, &mut blob_dump).unwrap();
        assert_eq!(size, pi.blob.data.length);
        assert_eq!(blob_dump.len(), size as usize);
    }

    {
        let img = &images[3];
        assert_eq!(
            img.guid.as_deref(),
            Some("{0711C6FD-1363-4F2E-CCBD-089F45CA2288}")
        );
        assert_eq!(img.name.as_deref(), Some("cylindrical"));
        let projection = img.projection.as_ref().unwrap();
        assert!(matches!(projection, Projection::Cylindrical(..)));
        let ci = if let Projection::Cylindrical(value) = projection {
            value
        } else {
            panic!("Unexpected projection")
        };
        assert_eq!(ci.properties.height, 100);
        assert_eq!(ci.properties.width, 100);
        assert_eq!(ci.properties.pixel_width, 0.044);
        assert_eq!(ci.properties.pixel_height, 0.033);
        assert_eq!(ci.properties.principal_y, 42.0);
        assert_eq!(ci.properties.radius, 666.0);
        assert_eq!(ci.blob.data.offset, 10076);
        assert_eq!(ci.blob.data.length, 1073);
        assert!(matches!(ci.blob.format, ImageFormat::Png));
        assert_eq!(ci.properties.width, 100);
        assert_eq!(ci.properties.height, 100);
        let mut blob_dump = Vec::new();
        let size = reader.blob(&ci.blob.data, &mut blob_dump).unwrap();
        assert_eq!(size, ci.blob.data.length);
        assert_eq!(blob_dump.len(), size as usize);
    }
}

#[test]
fn read_empty_pc() {
    let path = "testdata/empty_pc.e57";
    let mut e57 = E57Reader::from_file(path).unwrap();
    assert_eq!(e57.guid(), "{3DA87555-C99B-42CF-FAB5-F994D4F98235}");
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    for pc in pointclouds {
        assert_eq!(
            pc.guid.as_deref(),
            Some("{509F3BEA-9555-4667-5608-266CC699CA43}")
        );
        assert_eq!(pc.prototype.len(), 3);
        assert_eq!(pc.records, 0);
        let iter = e57.pointcloud_raw(&pc).unwrap();
        let points: Result<Vec<RawValues>> = iter.collect();
        let points = points.unwrap();
        assert_eq!(points.len(), 0);
    }
}

#[test]
fn integer_intensity() {
    let path = "testdata/integer_intensity.e57";
    let mut e57 = E57Reader::from_file(path).unwrap();
    assert_eq!(e57.guid(), "{7B6300FA-DFC7-4023-EBCC-048E36EF7E47}");
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    for pc in pointclouds {
        assert_eq!(
            pc.guid.as_deref(),
            Some("{B774562C-6E97-421B-F5FD-F9BFF4DC0DED}")
        );
        assert_eq!(pc.prototype.len(), 5);
        assert_eq!(pc.records, 2);

        let iter = e57.pointcloud_simple(&pc).unwrap();
        let p: Result<Vec<Point>> = iter.collect();
        let p = p.unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].intensity.unwrap(), 0.0);
        assert_eq!(p[1].intensity.unwrap(), 1.0);

        // Order of raw values: X, Y, Z, I, CIS
        let iter = e57.pointcloud_raw(&pc).unwrap();
        let points: Result<Vec<RawValues>> = iter.collect();
        let points = points.unwrap();
        let proto = &pc.prototype;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0][0].to_f64(&proto[0].data_type).unwrap(), 1.1);
        assert_eq!(points[0][1].to_f64(&proto[1].data_type).unwrap(), 2.2);
        assert_eq!(points[0][2].to_f64(&proto[2].data_type).unwrap(), 3.3);
        assert_eq!(points[0][3].to_i64(&proto[3].data_type).unwrap(), -66);
        assert_eq!(points[0][4].to_i64(&proto[4].data_type).unwrap(), 0);
        assert_eq!(points[1][0].to_f64(&proto[0].data_type).unwrap(), 4.4);
        assert_eq!(points[1][1].to_f64(&proto[1].data_type).unwrap(), 5.5);
        assert_eq!(points[1][2].to_f64(&proto[2].data_type).unwrap(), 6.6);
        assert_eq!(points[1][3].to_i64(&proto[3].data_type).unwrap(), 66);
        assert_eq!(points[1][4].to_i64(&proto[4].data_type).unwrap(), 0);
    }
}

#[test]
fn scaled_integer_intensity() {
    let path = "testdata/scaled_integer_intensity.e57";
    let mut e57 = E57Reader::from_file(path).unwrap();
    assert_eq!(e57.guid(), "{551290DB-3BC5-4471-AD68-11105F07AC03}");
    let pointclouds = e57.pointclouds();
    assert_eq!(pointclouds.len(), 1);
    for pc in pointclouds {
        assert_eq!(
            pc.guid.as_deref(),
            Some("{1B49982E-3706-4A88-FCF2-06DB38E7A155}")
        );
        assert_eq!(pc.prototype.len(), 5);
        assert_eq!(pc.records, 2);

        let iter = e57.pointcloud_simple(&pc).unwrap();
        let points: Result<Vec<Point>> = iter.collect();
        let points = points.unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].intensity.unwrap(), 0.0);
        assert_eq!(points[1].intensity.unwrap(), 1.0);

        // Order of raw values: X, Y, Z, I, CIS
        let iter = e57.pointcloud_raw(&pc).unwrap();
        let p: Result<Vec<RawValues>> = iter.collect();
        let p = p.unwrap();
        let proto = &pc.prototype;
        assert_eq!(p.len(), 2);
        assert_eq!(p[0][0].to_f64(&proto[0].data_type).unwrap(), 1.1);
        assert_eq!(p[0][1].to_f64(&proto[1].data_type).unwrap(), 2.2);
        assert_eq!(p[0][2].to_f64(&proto[2].data_type).unwrap(), 3.3);
        assert_eq!(
            p[0][3].to_f64(&proto[3].data_type).unwrap(),
            -66.60000000000001
        );
        assert_eq!(p[0][4].to_i64(&proto[4].data_type).unwrap(), 0);
        assert_eq!(p[1][0].to_f64(&proto[0].data_type).unwrap(), 4.4);
        assert_eq!(p[1][1].to_f64(&proto[1].data_type).unwrap(), 5.5);
        assert_eq!(p[1][2].to_f64(&proto[2].data_type).unwrap(), 6.6);
        assert_eq!(
            p[1][3].to_f64(&proto[3].data_type).unwrap(),
            66.60000000000001
        );
        assert_eq!(p[1][4].to_i64(&proto[4].data_type).unwrap(), 0);
    }
}

#[test]
fn no_images_tag() {
    let e57 = E57Reader::from_file("testdata/las2e57_no_images_tag.e57").unwrap();
    assert_eq!(e57.images().len(), 0);
}

#[test]
fn las2e57() {
    let mut e57 = E57Reader::from_file("testdata/las2e57_no_images_tag.e57").unwrap();

    // Extension should be detected
    let extentions = e57.extensions();
    assert_eq!(extentions.len(), 1);
    assert_eq!(extentions[0].namespace, "las");

    // Point cloud prototype should contain LAS record
    let pcs = e57.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert!(matches!(
        pc.prototype[5].data_type,
        RecordDataType::Integer { min: 0, max: 65535 }
    ));
    assert_eq!(
        pc.prototype[5].name,
        RecordName::Unknown {
            namespace: String::from("las"),
            name: String::from("pointSourceId")
        }
    );

    // Reading all the LAS record values should work
    let iter = e57.pointcloud_raw(pc).unwrap();
    let points = iter.collect::<Result<Vec<RawValues>>>().unwrap();
    assert_eq!(points.len(), pc.records as usize);
    let point = points.first().unwrap().clone();
    assert_eq!(point[5], RecordValue::Integer(1));
}
