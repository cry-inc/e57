use crate::error::Converter;
use crate::iterator::pointcloud_iterator;
use crate::paged_reader::PagedReader;
use crate::pointcloud::pointclouds_from_document;
use crate::root::root_from_document;
use crate::root::Root;
use crate::DateTime;
use crate::Error;
use crate::Header;
use crate::PointCloud;
use crate::PointCloudIterator;
use crate::Result;
use roxmltree::Document;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::path::Path;

const MAX_XML_SIZE: usize = 1024 * 1024 * 10;

/// Main interface for dealing with E57 files.
pub struct E57<T: Read + Seek> {
    reader: PagedReader<T>,
    header: Header,
    xml: String,
    root: Root,
    pointclouds: Vec<PointCloud>,
}

impl<T: Read + Seek> E57<T> {
    /// Creates a new E57 instance for from a reader.
    pub fn from_reader(mut reader: T) -> Result<Self> {
        // Read binary file header
        let mut header_bytes = [0_u8; 48];
        reader
            .read_exact(&mut header_bytes)
            .read_err("Failed to read file header")?;

        // Parse and validate E57 header
        let header = Header::from_array(&header_bytes)?;

        // Set up paged reader for the CRC page layer
        let mut reader = PagedReader::new(reader, header.page_size)
            .read_err("Failed creating paged CRC reader")?;

        // Read and parse XML data
        let xml_raw = Self::extract_xml(
            &mut reader,
            header.phys_xml_offset,
            header.xml_length as usize,
        )?;
        let xml = String::from_utf8(xml_raw).read_err("Failed to parse XML as UTF8")?;
        let document = Document::parse(&xml).invalid_err("Failed to parse XML data")?;
        let root = root_from_document(&document)?;
        let pointclouds = pointclouds_from_document(&document)?;

        Ok(Self {
            reader,
            header,
            xml,
            root,
            pointclouds,
        })
    }

    /// Returns the contents of E57 binary file header structure.
    pub fn header(&self) -> Header {
        self.header.clone()
    }

    /// Returns the XML section of the E57 file.
    pub fn xml(&self) -> &str {
        &self.xml
    }

    /// Returns format name stored in the XML section.
    pub fn format_name(&self) -> &str {
        &self.root.format
    }

    /// Returns GUID stored in the XML section.
    pub fn guid(&self) -> &str {
        &self.root.guid
    }

    /// Returns a list of all point clouds in the file.
    pub fn pointclouds(&self) -> Vec<PointCloud> {
        self.pointclouds.clone()
    }

    /// Returns an iterator for the requested point cloud.
    pub fn pointcloud(&mut self, pc: &PointCloud) -> Result<PointCloudIterator<T>> {
        pointcloud_iterator(pc, &mut self.reader)
    }

    /// Returns the optional creation date and time of the file.
    pub fn creation(&self) -> Option<DateTime> {
        self.root.creation.clone()
    }

    /// Returns the optional coordinate system metadata.
    ///
    /// This should contain a Coordinate Reference System that is specified by
    /// a string in a well-known text format for a spatial reference system,
    /// as defined by the Coordinate Transformation Service specification
    /// developed by the Open Geospatial Consortium.
    /// See also: <https://www.ogc.org/standard/wkt-crs/>
    pub fn coordinate_metadata(&self) -> Option<&str> {
        self.root.coordinate_metadata.as_deref()
    }

    /// Iterate over an reader to check an E57 file for CRC errors.
    ///
    /// This standalone function does only the minimal parsing required
    /// to get the E57 page size and without any other checks or validation.
    /// After that it will CRC-validate the whole file.
    /// It will not read or check any other file header and XML data!
    /// This method returns the page size of the E57 file.
    pub fn validate_crc(mut reader: T) -> Result<u64> {
        let page_size = Self::get_u64(&mut reader, 40, "page size")?;
        let mut paged_reader =
            PagedReader::new(reader, page_size).read_err("Failed creating paged CRC reader")?;
        let mut buffer = vec![0_u8; page_size as usize];
        let mut page = 0;
        while paged_reader
            .read(&mut buffer)
            .read_err(format!("Failed to validate CRC for page {page}"))?
            != 0
        {
            page += 1;
        }
        Ok(page_size)
    }

    /// Returns the raw unparsed binary XML data of the E57 file as bytes.
    ///
    /// This standalone function does only the minimal parsing required
    /// to get the XML section without any other checks or any other
    /// validation than basic CRC ckecking for the XML section itself.
    pub fn raw_xml(mut reader: T) -> Result<Vec<u8>> {
        let page_size = Self::get_u64(&mut reader, 40, "page size")?;
        let xml_offset = Self::get_u64(&mut reader, 24, "XML offset")?;
        let xml_length = Self::get_u64(&mut reader, 32, "XML length")?;

        // Create paged CRC reader
        let mut paged_reader =
            PagedReader::new(reader, page_size).read_err("Failed creating paged CRC reader")?;

        // Read XML data
        Self::extract_xml(&mut paged_reader, xml_offset, xml_length as usize)
    }

    fn get_u64(reader: &mut T, offset: u64, name: &str) -> Result<u64> {
        reader
            .seek(std::io::SeekFrom::Start(offset))
            .read_err(format!("Cannot seek to {name} offset"))?;
        let mut buf = [0_u8; 8];
        reader
            .read_exact(&mut buf)
            .read_err(format!("Cannot read {name} bytes"))?;
        Ok(u64::from_le_bytes(buf))
    }

    fn extract_xml(reader: &mut PagedReader<T>, offset: u64, length: usize) -> Result<Vec<u8>> {
        if length > MAX_XML_SIZE {
            Error::not_implemented(format!(
                "XML sections larger than {MAX_XML_SIZE} bytes are not supported"
            ))?
        }
        reader
            .seek_physical(offset)
            .read_err("Cannot seek to XML offset")?;
        let mut xml = vec![0_u8; length];
        reader
            .read_exact(&mut xml)
            .read_err("Failed to read XML data")?;
        Ok(xml)
    }
}

impl E57<BufReader<File>> {
    /// Creates an E57 instance from a Path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path).read_err("Unable to open file")?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LimitValue, Point, Record};
    use std::io::{BufWriter, Write};

    #[test]
    fn header() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();

        let header = reader.header();
        assert_eq!(header.major, 1);
        assert_eq!(header.minor, 0);
        assert_eq!(header.page_size, 1024);
    }

    #[test]
    fn validate() {
        let file = File::open("testdata/bunnyDouble.e57").unwrap();
        assert_eq!(E57::validate_crc(file).unwrap(), 1024);

        let file = File::open("testdata/corrupt_crc.e57").unwrap();
        assert!(E57::validate_crc(file).is_err());
    }

    #[test]
    fn xml() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let header = reader.header();
        let xml = reader.xml();
        assert_eq!(xml.as_bytes().len(), header.xml_length as usize);
    }

    #[test]
    fn raw_xml() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let header = reader.header();

        let reader = File::open("testdata/bunnyDouble.e57").unwrap();
        let xml = E57::raw_xml(reader).unwrap();
        assert_eq!(xml.len(), header.xml_length as usize);
    }

    #[test]
    fn format_name() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let format = reader.format_name();
        assert_eq!(format, "ASTM E57 3D Imaging Data File");
    }

    #[test]
    fn guid() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let guid = reader.guid();
        assert_eq!(guid, "{19AA90ED-145E-4B3B-922C-80BC00648844}");
    }

    #[test]
    fn creation() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let creation = reader.creation().unwrap();
        assert_eq!(creation.gps_time, 987369380.8049808);
        assert_eq!(creation.atomic_reference, false);
    }

    #[test]
    fn pointclouds() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let pcs = reader.pointclouds();
        assert_eq!(pcs.len(), 1);
        let pc = pcs.first().unwrap();
        assert_eq!(pc.guid, "{9CA24C38-C93E-40E8-A366-F49977C7E3EB}");
        assert_eq!(pc.name, Some(String::from("bunny")));
        assert_eq!(pc.file_offset, 48);
        assert_eq!(pc.records, 30571);
        assert_eq!(pc.prototype.len(), 4);
        assert!(matches!(pc.prototype[0], Record::CartesianX { .. }));
        assert!(matches!(pc.prototype[1], Record::CartesianY { .. }));
        assert!(matches!(pc.prototype[2], Record::CartesianZ { .. }));
        assert!(matches!(
            pc.prototype[3],
            Record::CartesianInvalidState { .. }
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
            let mut reader = E57::from_file(file).unwrap();
            let pcs = reader.pointclouds();
            let pc = pcs.first().unwrap();
            let points: Vec<Point> = reader.pointcloud(pc).unwrap().map(|p| p.unwrap()).collect();
            assert_eq!(points.len(), 30571);
        }
    }

    #[test]
    fn cartesian_bounds() {
        let file = "testdata/tinyCartesianFloatRgb.e57";
        let reader = E57::from_file(file).unwrap();
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
        let reader = E57::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        let limits = pc.color_limits.as_ref().unwrap();
        assert_eq!(limits.red_min, Some(LimitValue::Integer(0)));
        assert_eq!(limits.red_max, Some(LimitValue::Integer(255)));
        assert_eq!(limits.green_min, Some(LimitValue::Integer(0)));
        assert_eq!(limits.green_max, Some(LimitValue::Integer(255)));
        assert_eq!(limits.blue_min, Some(LimitValue::Integer(0)));
        assert_eq!(limits.blue_max, Some(LimitValue::Integer(255)));
    }

    #[test]
    fn simple_iterator_test() {
        let file = "testdata/tinyCartesianFloatRgb.e57";
        let mut reader = E57::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        let mut counter = 0;
        for p in reader.pointcloud(pc).unwrap() {
            let p = p.unwrap();
            assert!(p.cartesian.is_some());
            assert!(p.color.is_some());
            counter += 1;
        }
        assert_eq!(counter, pc.records);
    }

    #[test]
    #[ignore]
    fn debug() {
        let mut reader = E57::from_file("testdata/bunnyInt19.e57").unwrap();
        std::fs::write("dump.xml", reader.xml()).unwrap();

        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        let writer = File::create("dump.xyz").unwrap();
        let mut writer = BufWriter::new(writer);
        for p in reader.pointcloud(pc).unwrap() {
            let p = p.unwrap();
            if let Some(c) = p.cartesian {
                if let Some(invalid) = p.cartesian_invalid {
                    if invalid != 0 {
                        continue;
                    }
                }
                writer
                    .write_fmt(format_args!("{} {} {}", c.x, c.y, c.z))
                    .unwrap();
            } else if let Some(s) = p.spherical {
                if let Some(invalid) = p.spherical_invalid {
                    if invalid != 0 {
                        continue;
                    }
                }
                let cos_ele = f64::cos(s.elevation);
                let x = s.range * cos_ele * f64::cos(s.azimuth);
                let y = s.range * cos_ele * f64::sin(s.azimuth);
                let z = s.range * f64::sin(s.elevation);
                writer.write_fmt(format_args!("{x} {y} {z}")).unwrap();
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
            } else if let Some(intensity) = p.intensity {
                writer
                    .write_fmt(format_args!(
                        " {} {} {}",
                        (intensity * 255.) as u8,
                        (intensity * 255.) as u8,
                        (intensity * 255.) as u8
                    ))
                    .unwrap();
            }
            writer.write_fmt(format_args!("\n")).unwrap();
        }
    }
}
