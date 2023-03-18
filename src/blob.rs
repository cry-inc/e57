use crate::error::{Converter, WRONG_OFFSET};
use crate::paged_reader::PagedReader;
use crate::{Error, Result};
use roxmltree::Node;
use std::io::{copy, Read, Seek, Write};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Blob {
    pub offset: u64,
    pub length: u64,
}

pub fn blob_from_node(node: &Node) -> Result<Blob> {
    if Some("Blob") != node.attribute("type") {
        Error::invalid("The supplided tag is not a blob")?
    }

    let offset = node
        .attribute("fileOffset")
        .invalid_err("Failed to find 'fileOffset' attribute in blob tag")?;
    let offset = offset
        .parse::<u64>()
        .invalid_err("Unable to parse offset as u64")?;

    let length = node
        .attribute("length")
        .invalid_err("Failed to find 'length' attribute in blob tag")?;
    let length = length
        .parse::<u64>()
        .invalid_err("Unable to parse length as u64")?;

    Ok(Blob { offset, length })
}

pub fn extract_blob<T: Read + Seek>(
    reader: &mut PagedReader<T>,
    blob: &Blob,
    writer: &mut dyn Write,
) -> Result<u64> {
    reader
        .seek_physical(blob.offset)
        .read_err("Failed to seek to start offset of blob")?;
    let header = BlobSectionHeader::from_reader(reader)?;
    if blob.length > header.section_length + 16 {
        Error::invalid("Blob XML length and blob section header mismatch")?
    }

    let mut limited = reader.take(blob.length);
    copy(&mut limited, writer).read_err("Failed to read binary blob data")
}

#[derive(Debug)]
struct BlobSectionHeader {
    _section_id: u8,
    section_length: u64,
}

impl BlobSectionHeader {
    pub fn from_array(buffer: &[u8; 16]) -> Result<Self> {
        if buffer[0] != 0 {
            Error::invalid("Section ID of the blob section header is not 0")?
        }
        Ok(Self {
            _section_id: buffer[0],
            section_length: u64::from_le_bytes(
                buffer[8..16].try_into().internal_err(WRONG_OFFSET)?,
            ),
        })
    }

    fn from_reader<T: Read + Seek>(reader: &mut PagedReader<T>) -> Result<BlobSectionHeader> {
        let mut buffer = [0_u8; 16];
        reader
            .read_exact(&mut buffer)
            .read_err("Failed to read compressed vector section header")?;
        BlobSectionHeader::from_array(&buffer)
    }
}
