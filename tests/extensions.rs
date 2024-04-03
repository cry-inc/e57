use e57::{Blob, E57Reader, E57Writer, Error, Extension, RawValues};
use e57::{Record, RecordDataType, RecordName, RecordValue};
use roxmltree::Document;
use std::fs::remove_file;
use std::io::Cursor;
use std::path::Path;

// This test demonstrates how to write and read E57 files with custom extensions.
// Please check the documentation at <docs.rs/e57> for additional information!
#[test]
fn extensions_example() {
    let path = Path::new("extension_example.e57");

    {
        // Write an E57 file with extension
        let mut writer = E57Writer::from_file(path, "file_guid").unwrap();

        // Define extension
        let ext = Extension {
            namespace: "myext".to_owned(),
            url: "https://www.mycorp.com/myext".to_owned(),
        };

        // Register extension
        writer.register_extension(ext.clone()).unwrap();

        // Define point cloud prototype with XYZ and custom classification attribute
        let prototype = vec![
            Record::CARTESIAN_X_F32,
            Record::CARTESIAN_Y_F32,
            Record::CARTESIAN_Z_F32,
            Record {
                name: RecordName::Unknown {
                    namespace: ext.namespace.clone(),
                    name: String::from("classification"),
                },
                data_type: RecordDataType::Integer { min: 0, max: 10 },
            },
        ];

        // Add example point cloud with extension attribute
        let mut pc_writer = writer.add_pointcloud("pc_guid", prototype).unwrap();
        pc_writer
            .add_point(vec![
                RecordValue::Single(1.0),
                RecordValue::Single(2.0),
                RecordValue::Single(3.0),
                RecordValue::Integer(9),
            ])
            .unwrap();
        pc_writer.finalize().unwrap();

        // Add additional binary data to the E57 file
        let data: Vec<u8> = vec![1, 3, 3, 7];
        let mut cursor = Cursor::new(data);
        let blob = writer.add_blob(&mut cursor).unwrap();

        // Prepare custom XML tag for blob data
        let blob_xml = format!(
            "<myext:myblob type=\"Structure\" offset=\"{}\" length=\"{}\"></myext:myblob>",
            blob.offset, blob.length
        );

        // Finalize file and inject additional XML tag using transformer closure
        let transformer = |xml: String| {
            let old = "</e57Root>";
            let new = format!("{}\n</e57Root>", blob_xml);
            Ok(xml.replace(old, &new))
        };
        writer.finalize_customized_xml(transformer).unwrap();
    }

    {
        // Open E57 file with extenstion for reading
        let mut e57 = E57Reader::from_file(path).unwrap();

        // Check extensions registered as XML namespaces
        let extensions = e57.extensions();
        assert_eq!(extensions.len(), 1);
        let ext = extensions.first().unwrap();
        assert_eq!(ext.namespace, "myext");
        assert_eq!(ext.url, "https://www.mycorp.com/myext");

        // Get point cloud and check for custom attribute
        let pointclouds = e57.pointclouds();
        assert_eq!(pointclouds.len(), 1);
        let pointcloud = pointclouds.first().unwrap();
        let custom_record = &pointcloud.prototype[3];
        assert_eq!(
            custom_record.name,
            RecordName::Unknown {
                namespace: String::from("myext"),
                name: String::from("classification")
            }
        );

        // Read point data and check custom attribute value
        let points = e57
            .pointcloud_raw(pointcloud)
            .unwrap()
            .collect::<Result<Vec<RawValues>, Error>>()
            .unwrap();
        assert_eq!(points.len(), 1);
        let point = points.first().unwrap();
        assert_eq!(point[3], RecordValue::Integer(9));

        // Get custom binary blob metadata from XML using roxmltree
        let xml = e57.xml();
        let document = Document::parse(xml).unwrap();
        let blob = document
            .descendants()
            .find(|node| node.has_tag_name("myblob"))
            .unwrap();
        let offset = blob.attribute("offset").unwrap().parse::<u64>().unwrap();
        let length = blob.attribute("length").unwrap().parse::<u64>().unwrap();

        // Read blob data from E57 file
        let blob = Blob::new(offset, length);
        let mut data = Vec::new();
        e57.blob(&blob, &mut data).unwrap();
        assert_eq!(data.len(), 4);
    }

    remove_file(path).unwrap();
}
