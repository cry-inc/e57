use crate::paged_reader::PagedReader;
use crate::queue_reader::QueueReader;
use crate::PointCloud;
use crate::RawValues;
use crate::Result;
use std::io::{Read, Seek};

/// Iterate over all raw points of a point cloud for reading.
pub struct PointCloudReaderRaw<'a, T: Read + Seek> {
    queue_reader: QueueReader<'a, T>,
    prototype_len: usize,
    records: u64,
    read: u64,
}

impl<'a, T: Read + Seek> PointCloudReaderRaw<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        let queue_reader = QueueReader::new(pc, reader)?;
        let prototype_len = pc.prototype.len();
        let records = pc.records;
        Ok(Self {
            queue_reader,
            prototype_len,
            records,
            read: 0,
        })
    }
}

impl<T: Read + Seek> Iterator for PointCloudReaderRaw<'_, T> {
    /// Each iterator item is a result for an extracted point.
    type Item = Result<RawValues>;

    /// Returns the next available point or None if the end was reached.
    fn next(&mut self) -> Option<Self::Item> {
        // Already read all points?
        if self.read >= self.records {
            return None;
        }

        // Refill property queues if required
        // (in some corner cases more than one advance is required)
        while self.queue_reader.available() < 1 {
            if let Err(err) = self.queue_reader.advance() {
                return Some(Err(err));
            }
        }

        // Extract next point
        let mut point = RawValues::with_capacity(self.prototype_len);
        match self.queue_reader.pop_point(&mut point) {
            Ok(()) => {
                self.read += 1;
                Some(Ok(point))
            }
            Err(err) => Some(Err(err)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let overall = self.records;
        let remaining = overall - self.read;
        (remaining as usize, Some(remaining as usize))
    }
}
