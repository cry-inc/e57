use crate::error::invalid_file_err;
use crate::error::read_err;
use crate::paged_reader::PagedReader;
use crate::xml::XmlDocument;
use crate::Header;
use crate::PointCloud;
use crate::Result;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::path::Path;

pub struct E57<T: Read + Seek> {
    reader: PagedReader<T>,
    header: Header,
    xml: XmlDocument,
}

impl<T: Read + Seek> E57<T> {
    /// Creates a new E57 instance for from a reader.
    pub fn from_reader(mut reader: T) -> Result<Self> {
        let mut header_bytes = [0_u8; 48];
        reader
            .read_exact(&mut header_bytes)
            .map_err(|e| read_err("Failed to read 48 byte file header", e))?;

        // Parse and validate E57 header
        let header = Header::from_bytes(&header_bytes)?;

        // Set up paged reader for the CRC page layer
        let mut reader = PagedReader::new(reader, header.page_size)
            .map_err(|e| invalid_file_err("Unable to setup CRC reader for E57 file", e))?;

        // Read XML section
        reader
            .seek_physical(header.phys_xml_offset)
            .map_err(|e| read_err("Failed to seek to XML section", e))?;
        let mut xml = vec![0_u8; header.xml_length as usize];
        reader
            .read_exact(&mut xml)
            .map_err(|e| read_err("Failed to read XML section", e))?;

        // Parse XML data
        let xml = String::from_utf8(xml)
            .map_err(|e| invalid_file_err("Failed to parse XML as UTF8 string", e))?;
        let xml = XmlDocument::parse(xml)?;

        Ok(Self {
            reader,
            header,
            xml,
        })
    }

    /// Returns the E57 file header structure.
    pub fn get_header(&self) -> Header {
        self.header.clone()
    }

    /// Iterate over the whole file to check for CRC errors.
    pub fn validate_crc(&mut self) -> Result<()> {
        self.reader.rewind().unwrap();
        let mut buffer = vec![0_u8; self.header.page_size as usize];
        while self
            .reader
            .read(&mut buffer)
            .map_err(|e| read_err("Failed to read file for validation", e))?
            == 0
        {}
        Ok(())
    }

    /// Returns the raw XML data of the E57 file as bytes.
    pub fn raw_xml(&self) -> &str {
        self.xml.raw_xml()
    }

    /// Returns format name stored in the XML section.
    pub fn format_name(&self) -> Option<&str> {
        self.xml.format_name().map(|x| &**x)
    }

    /// Returns GUID stored in the XML section.
    pub fn guid(&self) -> Option<&str> {
        self.xml.guid().map(|x| &**x)
    }

    /// Returns a list of all point clouds in the file.
    pub fn pointclouds(&self) -> Vec<PointCloud> {
        self.xml.pointclouds()
    }
}

impl E57<File> {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path).map_err(|e| read_err("Unable to open file", e))?;
        Self::from_reader(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Record;

    #[test]
    fn header() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();

        let header = reader.get_header();
        assert_eq!(header.major, 1);
        assert_eq!(header.minor, 0);
        assert_eq!(header.page_size, 1024);
    }

    #[test]
    fn validate() {
        let mut reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        reader.validate_crc().unwrap();
    }

    #[test]
    fn raw_xml() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let header = reader.get_header();
        let xml = reader.raw_xml();
        assert_eq!(xml.len() as u64, header.xml_length);
        //std::fs::write("dump.xml", xml).unwrap();
    }

    #[test]
    fn format_name() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let format = reader.format_name();
        assert_eq!(format, Some("ASTM E57 3D Imaging Data File"));
    }

    #[test]
    fn guid() {
        let reader = E57::from_file("testdata/bunnyDouble.e57").unwrap();
        let guid = reader.guid();
        assert_eq!(guid, Some("{19AA90ED-145E-4B3B-922C-80BC00648844}"));
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
}
