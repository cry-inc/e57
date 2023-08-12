use crate::paged_reader::PagedReader;
use crate::{
    CartesianCoordinate, Color, Point, PointCloud, PointCloudReaderRaw, RawValues, RecordName,
    Result, SphericalCoordinate, Transform, Translation,
};
use std::io::{Read, Seek};

struct ValueIndices {
    cartesian: Option<(usize, usize, usize)>,
    cartesian_invalid: Option<usize>,
    spherical: Option<(usize, usize, usize)>,
    spherical_invalid: Option<usize>,
    color: Option<(usize, usize, usize)>,
    color_invalid: Option<usize>,
    intensity: Option<usize>,
    intensity_invalid: Option<usize>,
    row: Option<usize>,
    column: Option<usize>,
}

/// Iterate over all normalized points of a point cloud for reading.
pub struct PointCloudReaderSimple<'a, T: Read + Seek> {
    pc: PointCloud,
    raw_iter: PointCloudReaderRaw<'a, T>,
    skip: bool,
    transform: bool,
    s2c: bool,
    i2c: bool,
    rotation: [f64; 9],
    translation: Translation,
    indices: ValueIndices,
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

        // Prepare indices for fast value lookup
        let cx = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::CartesianX);
        let cy = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::CartesianY);
        let cz = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::CartesianZ);
        let cartesian = match (cx, cy, cz) {
            (Some(cx), Some(cy), Some(cz)) => Some((cx, cy, cz)),
            _ => None,
        };
        let cartesian_invalid = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::CartesianInvalidState);
        let sr = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::SphericalRange);
        let sa = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::SphericalAzimuth);
        let se = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::SphericalElevation);
        let spherical = match (sr, sa, se) {
            (Some(sr), Some(sa), Some(se)) => Some((sr, sa, se)),
            _ => None,
        };
        let spherical_invalid = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::SphericalInvalidState);
        let red = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::ColorRed);
        let green = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::ColorGreen);
        let blue = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::ColorBlue);
        let color = match (red, green, blue) {
            (Some(red), Some(green), Some(blue)) => Some((red, green, blue)),
            _ => None,
        };
        let color_invalid = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::IsColorInvalid);
        let intensity = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::Intensity);
        let intensity_invalid = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::IsIntensityInvalid);
        let row = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::RowIndex);
        let column = pc
            .prototype
            .iter()
            .position(|r| r.name == RecordName::ColumnIndex);
        let indices = ValueIndices {
            cartesian,
            cartesian_invalid,
            spherical,
            spherical_invalid,
            color,
            color_invalid,
            intensity,
            intensity_invalid,
            row,
            column,
        };

        Ok(Self {
            pc: pc.clone(),
            raw_iter: PointCloudReaderRaw::new(pc, reader)?,
            skip: false,
            transform: true,
            s2c: true,
            i2c: true,
            rotation,
            translation,
            indices,
        })
    }

    /// If enabled, the iterator will automatically convert spherical to Cartesian coordinates.
    /// Will only replace fully invalid cartesian coordinates and do nothing otherwise.
    /// Default setting is enabled.
    pub fn spherical_to_cartesian(&mut self, enable: bool) {
        self.s2c = enable;
    }

    /// If enabled, the iterator will automatically convert intensity to grey colors.
    /// Will only replace fully invalid color values and do nothing otherwise.
    /// Default setting is enabled.
    pub fn intensity_to_color(&mut self, enable: bool) {
        self.i2c = enable;
    }

    /// If enabled, the iterator will skip over points without valid Cartesian coordinates.
    /// Default setting is disabled, meaning the iterator will visit invalid points.
    pub fn skip_invalid(&mut self, enable: bool) {
        self.skip = enable;
    }

    /// If enabled, the iterator will apply the point cloud pose to the Cartesian coordinates.
    /// Default setting is enabled.
    pub fn apply_pose(&mut self, enable: bool) {
        self.transform = enable;
    }

    fn get_next_point(&mut self) -> Option<Result<Point>> {
        let p = self.raw_iter.next()?;
        match p {
            Ok(p) => Some(self.create_point(p)),
            Err(err) => Some(Err(err)),
        }
    }

    fn transform_point(&self, p: &mut Point) {
        if p.cartesian_invalid == 0 {
            let c = &mut p.cartesian;
            let x = self.rotation[0] * c.x + self.rotation[3] * c.y + self.rotation[6] * c.z;
            let y = self.rotation[1] * c.x + self.rotation[4] * c.y + self.rotation[7] * c.z;
            let z = self.rotation[2] * c.x + self.rotation[5] * c.y + self.rotation[8] * c.z;
            c.x = x + self.translation.x;
            c.y = y + self.translation.y;
            c.z = z + self.translation.z;
        }
    }

    fn convert_spherical(&self, p: &mut Point) {
        if p.spherical_invalid == 0 && p.cartesian_invalid != 0 {
            let cos_ele = f64::cos(p.spherical.elevation);
            p.cartesian = CartesianCoordinate {
                x: p.spherical.range * cos_ele * f64::cos(p.spherical.azimuth),
                y: p.spherical.range * cos_ele * f64::sin(p.spherical.azimuth),
                z: p.spherical.range * f64::sin(p.spherical.elevation),
            };
            p.cartesian_invalid = 0;
        }
    }

    fn convert_intensity(&self, p: &mut Point) {
        if p.intensity_invalid == 0 && p.color_invalid != 0 {
            p.color.red = p.intensity;
            p.color.green = p.intensity;
            p.color.blue = p.intensity;
            p.color_invalid = 0;
        }
    }

    fn create_point(&self, values: RawValues) -> Result<Point> {
        let prototype = &self.pc.prototype;

        let cartesian = if let Some(indices) = self.indices.cartesian {
            CartesianCoordinate {
                x: values[indices.0].to_f64(&prototype[indices.0].data_type)?,
                y: values[indices.1].to_f64(&prototype[indices.1].data_type)?,
                z: values[indices.2].to_f64(&prototype[indices.2].data_type)?,
            }
        } else {
            CartesianCoordinate::default()
        };
        let cartesian_invalid = if let Some(index) = self.indices.cartesian_invalid {
            values[index].to_u8(&prototype[index].data_type)?
        } else if self.indices.cartesian.is_some() {
            0
        } else {
            2
        };

        let spherical = if let Some(indices) = self.indices.spherical {
            SphericalCoordinate {
                range: values[indices.0].to_f64(&prototype[indices.0].data_type)?,
                azimuth: values[indices.1].to_f64(&prototype[indices.1].data_type)?,
                elevation: values[indices.2].to_f64(&prototype[indices.2].data_type)?,
            }
        } else {
            SphericalCoordinate::default()
        };
        let spherical_invalid = if let Some(index) = self.indices.spherical_invalid {
            values[index].to_u8(&prototype[index].data_type)?
        } else if self.indices.spherical.is_some() {
            0
        } else {
            2
        };

        let color = if let Some(indices) = self.indices.color {
            Color {
                red: values[indices.0].to_unit_f32(&prototype[indices.0].data_type)?,
                green: values[indices.1].to_unit_f32(&prototype[indices.1].data_type)?,
                blue: values[indices.2].to_unit_f32(&prototype[indices.2].data_type)?,
            }
        } else {
            Color::default()
        };
        let color_invalid = if let Some(index) = self.indices.color_invalid {
            values[index].to_u8(&prototype[index].data_type)?
        } else if self.indices.color.is_some() {
            0
        } else {
            1
        };

        let intensity = if let Some(index) = self.indices.intensity {
            values[index].to_unit_f32(&prototype[index].data_type)?
        } else {
            0.0
        };
        let intensity_invalid = if let Some(index) = self.indices.intensity_invalid {
            values[index].to_u8(&prototype[index].data_type)?
        } else if self.indices.intensity.is_some() {
            0
        } else {
            1
        };

        let row = if let Some(index) = self.indices.row {
            values[index].to_i64(&prototype[index].data_type)?
        } else {
            -1
        };
        let column = if let Some(index) = self.indices.column {
            values[index].to_i64(&prototype[index].data_type)?
        } else {
            -1
        };

        Ok(Point {
            cartesian,
            cartesian_invalid,
            spherical,
            spherical_invalid,
            color,
            color_invalid,
            intensity,
            intensity_invalid,
            row,
            column,
        })
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
                if self.s2c {
                    self.convert_spherical(&mut p);
                }
                if p.cartesian_invalid != 0 {
                    continue;
                }
                if self.i2c {
                    self.convert_intensity(&mut p);
                }
                if self.transform {
                    self.transform_point(&mut p);
                }
                return Some(Ok(p));
            }
        } else {
            match self.get_next_point()? {
                Ok(mut p) => {
                    if self.s2c {
                        self.convert_spherical(&mut p);
                    }
                    if self.i2c {
                        self.convert_intensity(&mut p);
                    }
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
