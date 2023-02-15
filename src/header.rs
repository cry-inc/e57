use crate::error::Converter;
use crate::error::WRONG_OFFSET;
use crate::Error;
use crate::Result;

const EXPECTED_SIGNATURE: &[u8] = "ASTM-E57".as_bytes();
const EXPECTED_MAJOR_VERSION: u32 = 1;
const EXPECTED_MINOR_VERSION: u32 = 0;
const EXPECTED_PAGE_SIZE: u64 = 1024;

/// Represents the file structure from the start of an E57 file.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Header {
    /// File header signature that must be always "ASTM-E57".
    pub signature: [u8; 8],

    /// Major version number of the E57 format of the file.
    pub major: u32,

    /// Minor version number of the E57 format of the file.
    pub minor: u32,

    /// Physical lenght of the E57 file on disk or in memory.
    pub phys_length: u64,

    /// Physical offset of the XML data inside the XML file.
    pub phys_xml_offset: u64,

    /// Logical (without CRC bytes) length of the XML data.
    pub xml_length: u64,

    /// Page size of the E57 file.
    pub page_size: u64,
}

impl Header {
    /// Creates an E57 file header structure from an array of bytes.
    pub fn from_array(data: &[u8; 48]) -> Result<Self> {
        let header = Header {
            signature: data[0..8].try_into().internal_err(WRONG_OFFSET)?,
            major: u32::from_le_bytes(data[8..12].try_into().internal_err(WRONG_OFFSET)?),
            minor: u32::from_le_bytes(data[12..16].try_into().internal_err(WRONG_OFFSET)?),
            phys_length: u64::from_le_bytes(data[16..24].try_into().internal_err(WRONG_OFFSET)?),
            phys_xml_offset: u64::from_le_bytes(
                data[24..32].try_into().internal_err(WRONG_OFFSET)?,
            ),
            xml_length: u64::from_le_bytes(data[32..40].try_into().internal_err(WRONG_OFFSET)?),
            page_size: u64::from_le_bytes(data[40..48].try_into().internal_err(WRONG_OFFSET)?),
        };

        if header.signature != EXPECTED_SIGNATURE {
            Error::invalid("Found unsupported signature in header")?
        }
        if header.major != EXPECTED_MAJOR_VERSION {
            Error::invalid("Found unsupported major version in header")?
        }
        if header.minor != EXPECTED_MINOR_VERSION {
            Error::invalid("Found unsupported minor version in header")?
        }
        if header.page_size != EXPECTED_PAGE_SIZE {
            Error::invalid("Found unsupported page size in header")?
        }

        Ok(header)
    }
}
