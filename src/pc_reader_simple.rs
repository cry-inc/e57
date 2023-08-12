use crate::paged_reader::PagedReader;
use crate::{Point, PointCloud, PointCloudReaderRaw, Result, Transform, Translation};
use std::io::{Read, Seek};

/// Iterate over all normalized points of a point cloud for reading.
pub struct PointCloudReaderSimple<'a, T: Read + Seek> {
    pc: PointCloud,
    raw_iter: PointCloudReaderRaw<'a, T>,
    skip: bool,
    transform: bool,
    convert: bool,
    rotation: [f64; 9],
    translation: Translation,
}

impl<'a, T: Read + Seek> PointCloudReaderSimple<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        // Prepare rotation and translation data
        let transform = pc.transform.clone().unwrap_or(Transform::default());
        let q = transform.rotation;
        let rotation = [
            q.w * q.w + q.x * q.x - q.y * q.y - q.z * q.z,
            2.0 * (q.x * q.y + q.w * q.z),
            2.0 * (q.x * q.z - q.w * q.y),
            2.0 * (q.x * q.y - q.w * q.z),
            q.w * q.w + q.y * q.y - q.x * q.x - q.z * q.z,
            2.0 * (q.y * q.z + q.w * q.x),
            2.0 * (q.x * q.z + q.w * q.y),
            2.0 * (q.y * q.z - q.w * q.x),
            q.w * q.w + q.z * q.z - q.x * q.x - q.y * q.y,
        ];
        let translation = transform.translation;

        Ok(Self {
            pc: pc.clone(),
            raw_iter: PointCloudReaderRaw::new(pc, reader)?,
            skip: false,
            transform: false,
            convert: false,
            rotation,
            translation,
        })
    }

    /// If enabled, the iterator will automatically convert spherical to Cartesian coordinates.
    /// Default setting is disabled, meaning the iterator will return no Cartesian coordinates for
    /// point clouds with spherical coordinates.
    pub fn convert_spherical(&mut self, enable: bool) {
        self.convert = enable;
    }

    /// If enabled, the iterator will skip over invalid points.
    /// Default setting is disabled, meaning the iterator will visit invalid points.
    pub fn skip_invalid(&mut self, enable: bool) {
        self.skip = enable;
    }

    /// If enabled, the iterator will apply the point cloud pose to the Cartesian coordinates.
    /// Default setting is disabled, meaning the iterator will return the unmodified Cartesian coordinates.
    pub fn apply_pose(&mut self, enable: bool) {
        self.transform = enable;
    }

    fn get_next_point(&mut self) -> Option<Result<Point>> {
        let p = self.raw_iter.next()?;
        match p {
            Ok(p) => Some(Point::from_values(p, &self.pc.prototype, self.convert)),
            Err(err) => Some(Err(err)),
        }
    }

    fn transform_point(&self, p: &mut Point) {
        if let Some(c) = &mut p.cartesian {
            let x = self.rotation[0] * c.x + self.rotation[3] * c.y + self.rotation[6] * c.z;
            let y = self.rotation[1] * c.x + self.rotation[4] * c.y + self.rotation[7] * c.z;
            let z = self.rotation[2] * c.x + self.rotation[5] * c.y + self.rotation[8] * c.z;
            c.x = x + self.translation.x;
            c.y = y + self.translation.y;
            c.z = z + self.translation.z;
        }
    }
}

impl<'a, T: Read + Seek> Iterator for PointCloudReaderSimple<'a, T> {
    /// Each iterator item is a result for an extracted point.
    type Item = Result<Point>;

    /// Returns the next available point or None if the end was reached.
    fn next(&mut self) -> Option<Self::Item> {
        if self.skip {
            loop {
                let p = self.get_next_point()?;
                let mut p = match p {
                    Ok(p) => p,
                    Err(err) => return Some(Err(err)),
                };
                if let Some(invalid) = p.cartesian_invalid {
                    if invalid != 0 {
                        continue;
                    }
                }
                if let Some(invalid) = p.spherical_invalid {
                    if p.cartesian.is_none() && invalid != 0 {
                        continue;
                    }
                }
                if self.transform {
                    self.transform_point(&mut p);
                }
                return Some(Ok(p));
            }
        } else {
            match self.get_next_point()? {
                Ok(mut p) => {
                    if self.transform {
                        self.transform_point(&mut p);
                    }
                    Some(Ok(p))
                }
                Err(err) => Some(Err(err)),
            }
        }
    }
}
