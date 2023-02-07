use crate::Error;

const EXPECTED_SIGNATURE: &[u8] = "ASTM-E57".as_bytes();
const EXPECTED_MAJOR_VERSION: u32 = 1;
const EXPECTED_MINOR_VERSION: u32 = 0;
const EXPECTED_PAGE_SIZE: u64 = 1024;

#[derive(Clone, Debug)]
pub struct Header {
    pub signature: [u8; 8],
    pub major: u32,
    pub minor: u32,
    pub phys_length: u64,
    pub phys_xml_offset: u64,
    pub xml_length: u64,
    pub page_size: u64,
}

impl Header {
    pub fn from_bytes(data: &[u8; 48]) -> Result<Self, Error> {
        let err = "Wrong header offsets detected, this is most likely a bug";
        let header = Header {
            signature: data[0..8].try_into().expect(err),
            major: u32::from_le_bytes(data[8..12].try_into().expect(err)),
            minor: u32::from_le_bytes(data[12..16].try_into().expect(err)),
            phys_length: u64::from_le_bytes(data[16..24].try_into().expect(err)),
            phys_xml_offset: u64::from_le_bytes(data[24..32].try_into().expect(err)),
            xml_length: u64::from_le_bytes(data[32..40].try_into().expect(err)),
            page_size: u64::from_le_bytes(data[40..48].try_into().expect(err)),
        };

        if header.signature != EXPECTED_SIGNATURE {
            Err(Error::InvalidFile(String::from(
                "Found unsupported signature in header",
            )))?;
        }
        if header.major != EXPECTED_MAJOR_VERSION {
            Err(Error::InvalidFile(String::from(
                "Found unsupported major version in header",
            )))?;
        }
        if header.minor != EXPECTED_MINOR_VERSION {
            Err(Error::InvalidFile(String::from(
                "Found unsupported minor version in header",
            )))?;
        }
        if header.page_size != EXPECTED_PAGE_SIZE {
            Err(Error::InvalidFile(String::from(
                "Found unsupported page size in header",
            )))?;
        }

        Ok(header)
    }
}
