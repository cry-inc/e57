use crate::error::Converter;
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

    fn pop_point(&mut self) -> Result<RawValues> {
        let mut point = RawValues::with_capacity(self.prototype_len);
        for i in 0..self.prototype_len {
            let value = self.queue_reader.queues[i]
                .pop_front()
                .internal_err("Failed to pop value for next point")?;
            point.push(value);
        }
        Ok(point)
    }
}

impl<'a, T: Read + Seek> Iterator for PointCloudReaderRaw<'a, T> {
    /// Each iterator item is a result for an extracted point.
    type Item = Result<RawValues>;

    /// Returns the next available point or None if the end was reached.
    fn next(&mut self) -> Option<Self::Item> {
        // Already read all points?
        if self.read >= self.records {
            return None;
        }

        // Refill property queues if required
        if self.queue_reader.available() < 1 {
            if let Err(err) = self.queue_reader.advance() {
                return Some(Err(err));
            }
        }

        // Extract next point
        match self.pop_point() {
            Ok(point) => {
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
