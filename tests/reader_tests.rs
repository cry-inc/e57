use e57::{CartesianCoordinate, E57Reader, Projection, RawValues, RecordName, RecordValue};
use std::fs::File;
use std::io::{BufWriter, Write};

#[test]
fn header() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();

    let header = reader.header();
    assert_eq!(header.major, 1);
    assert_eq!(header.minor, 0);
    assert_eq!(header.page_size, 1024);
}

#[test]
fn validate() {
    let file = File::open("testdata/bunnyDouble.e57").unwrap();
    assert_eq!(E57Reader::validate_crc(file).unwrap(), 1024);

    let file = File::open("testdata/corrupt_crc.e57").unwrap();
    assert!(E57Reader::validate_crc(file).is_err());
}

#[test]
fn xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();
    let xml = reader.xml();
    assert_eq!(xml.as_bytes().len(), header.xml_length as usize);
}

#[test]
fn raw_xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();

    let reader = File::open("testdata/bunnyDouble.e57").unwrap();
    let xml = E57Reader::raw_xml(reader).unwrap();
    assert_eq!(xml.len(), header.xml_length as usize);
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
fn pointclouds() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let pcs = reader.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert_eq!(pc.guid, "{9CA24C38-C93E-40E8-A366-F49977C7E3EB}");
    assert_eq!(pc.name, Some(String::from("bunny")));
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
        let points: Vec<RawValues> = reader
            .pointcloud_raw(pc)
            .unwrap()
            .map(|p| p.unwrap())
            .collect();
        assert_eq!(points.len(), 30571);
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
    assert_eq!(bounds.y_min, Some(4.5138792991638184));
    assert_eq!(bounds.y_max, Some(7.5154604911804199));
    assert_eq!(bounds.z_min, Some(295.52468872070312));
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
fn iterator() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
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
#[ignore]
fn debug_pointclouds() {
    let mut reader = E57Reader::from_file("testdata/bunnyInt19.e57").unwrap();
    std::fs::write("dump.xml", reader.xml()).unwrap();

    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let writer = File::create("dump.xyz").unwrap();
    let mut writer = BufWriter::new(writer);
    for p in reader.pointcloud_simple(pc).unwrap() {
        let p = p.unwrap();
        if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
            writer.write_fmt(format_args!("{x} {y} {z}",)).unwrap();
        } else {
            continue;
        }
        if let Some(color) = p.color {
            writer
                .write_fmt(format_args!(
                    " {} {} {}",
                    (color.red * 255.) as u8,
                    (color.green * 255.) as u8,
                    (color.blue * 255.) as u8
                ))
                .unwrap();
        }
        writer.write_fmt(format_args!("\n")).unwrap();
    }
}

#[test]
#[ignore]
fn debug_images() {
    let file = "./testdata/pumpA_visual_image.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    std::fs::write("dump.xml", reader.xml()).unwrap();
    let images = reader.images();
    for (index, img) in images.iter().enumerate() {
        println!("Image {index}: {img:#?}");
        if let Some(preview) = &img.visual_reference {
            let ext = format!("{:?}", preview.blob.format).to_lowercase();
            let filename = format!("preview_{index}.{ext}");
            let mut file = File::create(filename).unwrap();
            let size = reader.blob(&preview.blob.data, &mut file).unwrap();
            println!("Exported preview image with {size} bytes");
        }
        if let Some(rep) = &img.projection {
            let (blob, type_name) = match rep {
                Projection::Pinhole(rep) => (&rep.blob, "pinhole"),
                Projection::Spherical(rep) => (&rep.blob, "spherical"),
                Projection::Cylindrical(rep) => (&rep.blob, "cylindrical"),
            };
            let ext = format!("{:?}", blob.format).to_lowercase();
            let filename = format!("{type_name}_{index}.{ext}");
            let mut file = File::create(filename).unwrap();
            let size = reader.blob(&blob.data, &mut file).unwrap();
            println!("Exported image image with {size} bytes");
        }
    }
}
