use crate::paged_reader::PagedReader;
use crate::{Point, PointCloud, PointCloudReaderRaw, Result, Transform};
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use std::io::{Read, Seek};

/// Iterate over all normalized points of a point cloud for reading.
pub struct PointCloudReaderSimple<'a, T: Read + Seek> {
    pc: PointCloud,
    raw_iter: PointCloudReaderRaw<'a, T>,
    skip: bool,
    transform: bool,
    convert: bool,
    rotation: UnitQuaternion<f64>,
    translation: Vector3<f64>,
}

impl<'a, T: Read + Seek> PointCloudReaderSimple<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        // Prepare rotation and translation data
        let transform = pc.transform.clone().unwrap_or(Transform::default());
        let rotation = UnitQuaternion::from_quaternion(Quaternion::new(
            transform.rotation.w,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
        ));
        let translation = Vector3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        );

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

    /// If enabled, the iterator will automatically convert spherical to cartesian coordinates.
    /// Default setting is disabled, meaning the iterator will return no cartesian coordinates for
    /// point clouds with spherical coordinates.
    pub fn convert_spherical(&mut self, enable: bool) {
        self.convert = enable;
    }

    /// If enabled, the iterator will skip over invalid points.
    /// Default setting is disabled, meaning the iterator will visit invalid points.
    pub fn skip_invalid(&mut self, enable: bool) {
        self.skip = enable;
    }

    /// If enabled, the iterator will apply the point cloud pose to the cartesian coordinates.
    /// Default setting is disabled, meaning the iterator will return the unmodified cartesian coordinates.
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
        if let Some(cartesian) = &mut p.cartesian {
            let xyz = Point3::new(cartesian.x, cartesian.y, cartesian.z);
            let xyz = self.rotation.transform_point(&xyz) + self.translation;
            cartesian.x = xyz[0];
            cartesian.y = xyz[1];
            cartesian.z = xyz[2];
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
