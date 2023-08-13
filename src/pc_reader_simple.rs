use crate::paged_reader::PagedReader;
use crate::{
    CartesianCoordinate, Color, Point, PointCloud, PointCloudReaderRaw, RawValues, RecordName,
    Result, SphericalCoordinate, Transform, Translation,
};
use std::io::{Read, Seek};

struct Indices {
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
    indices: Indices,
}

impl<'a, T: Read + Seek> PointCloudReaderSimple<'a, T> {
    pub(crate) fn new(pc: &PointCloud, reader: &'a mut PagedReader<T>) -> Result<Self> {
        let (rotation, translation) = Self::prepare_transform(pc);
        let indices = Self::prepare_indices(pc);
        let raw_iter = PointCloudReaderRaw::new(pc, reader)?;
        Ok(Self {
            pc: pc.clone(),
            raw_iter,
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

    fn prepare_transform(pc: &PointCloud) -> ([f64; 9], Translation) {
        let t = if let Some(t) = &pc.transform {
            t.clone()
        } else {
            Transform::default()
        };
        let q = &t.rotation;
        (
            [
                q.w * q.w + q.x * q.x - q.y * q.y - q.z * q.z,
                2.0 * (q.x * q.y + q.w * q.z),
                2.0 * (q.x * q.z - q.w * q.y),
                2.0 * (q.x * q.y - q.w * q.z),
                q.w * q.w + q.y * q.y - q.x * q.x - q.z * q.z,
                2.0 * (q.y * q.z + q.w * q.x),
                2.0 * (q.x * q.z + q.w * q.y),
                2.0 * (q.y * q.z - q.w * q.x),
                q.w * q.w + q.z * q.z - q.x * q.x - q.y * q.y,
            ],
            t.translation,
        )
    }

    fn prepare_indices(pc: &PointCloud) -> Indices {
        let fi = |name: RecordName| -> Option<usize> {
            pc.prototype.iter().position(|r| r.name == name)
        };
        let cx = fi(RecordName::CartesianX);
        let cy = fi(RecordName::CartesianY);
        let cz = fi(RecordName::CartesianZ);
        let cartesian = match (cx, cy, cz) {
            (Some(cx), Some(cy), Some(cz)) => Some((cx, cy, cz)),
            _ => None,
        };
        let sr = fi(RecordName::SphericalRange);
        let sa = fi(RecordName::SphericalAzimuth);
        let se = fi(RecordName::SphericalElevation);
        let spherical = match (sr, sa, se) {
            (Some(sr), Some(sa), Some(se)) => Some((sr, sa, se)),
            _ => None,
        };
        let red = fi(RecordName::ColorRed);
        let green = fi(RecordName::ColorGreen);
        let blue = fi(RecordName::ColorBlue);
        let color = match (red, green, blue) {
            (Some(red), Some(green), Some(blue)) => Some((red, green, blue)),
            _ => None,
        };
        Indices {
            cartesian,
            cartesian_invalid: fi(RecordName::CartesianInvalidState),
            spherical,
            spherical_invalid: fi(RecordName::SphericalInvalidState),
            color,
            color_invalid: fi(RecordName::IsColorInvalid),
            intensity: fi(RecordName::Intensity),
            intensity_invalid: fi(RecordName::IsIntensityInvalid),
            row: fi(RecordName::RowIndex),
            column: fi(RecordName::ColumnIndex),
        }
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
        let proto = &self.pc.prototype;

        // Cartesian coordinates
        let cartesian = if let Some(ind) = self.indices.cartesian {
            CartesianCoordinate {
                x: values[ind.0].to_f64(&proto[ind.0].data_type)?,
                y: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                z: values[ind.2].to_f64(&proto[ind.2].data_type)?,
            }
        } else {
            CartesianCoordinate::default()
        };
        let cartesian_invalid = if let Some(ind) = self.indices.cartesian_invalid {
            values[ind].to_u8(&proto[ind].data_type)?
        } else if self.indices.cartesian.is_some() {
            0
        } else {
            2
        };

        // Spherical coordinates
        let spherical = if let Some(ind) = self.indices.spherical {
            SphericalCoordinate {
                range: values[ind.0].to_f64(&proto[ind.0].data_type)?,
                azimuth: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                elevation: values[ind.2].to_f64(&proto[ind.2].data_type)?,
            }
        } else {
            SphericalCoordinate::default()
        };
        let spherical_invalid = if let Some(ind) = self.indices.spherical_invalid {
            values[ind].to_u8(&proto[ind].data_type)?
        } else if self.indices.spherical.is_some() {
            0
        } else {
            2
        };

        // RGB colors
        let color = if let Some(ind) = self.indices.color {
            Color {
                red: values[ind.0].to_unit_f32(&proto[ind.0].data_type)?,
                green: values[ind.1].to_unit_f32(&proto[ind.1].data_type)?,
                blue: values[ind.2].to_unit_f32(&proto[ind.2].data_type)?,
            }
        } else {
            Color::default()
        };
        let color_invalid = if let Some(ind) = self.indices.color_invalid {
            values[ind].to_u8(&proto[ind].data_type)?
        } else if self.indices.color.is_some() {
            0
        } else {
            1
        };

        // Intensity values
        let intensity = if let Some(ind) = self.indices.intensity {
            values[ind].to_unit_f32(&proto[ind].data_type)?
        } else {
            0.0
        };
        let intensity_invalid = if let Some(ind) = self.indices.intensity_invalid {
            values[ind].to_u8(&proto[ind].data_type)?
        } else if self.indices.intensity.is_some() {
            0
        } else {
            1
        };

        // Row index
        let row = if let Some(ind) = self.indices.row {
            values[ind].to_i64(&proto[ind].data_type)?
        } else {
            -1
        };

        // Column index
        let column = if let Some(ind) = self.indices.column {
            values[ind].to_i64(&proto[ind].data_type)?
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
        loop {
            let mut p = match self.get_next_point()? {
                Ok(p) => p,
                Err(err) => return Some(Err(err)),
            };
            if self.s2c {
                self.convert_spherical(&mut p);
            }
            if self.skip && p.cartesian_invalid != 0 {
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
    }
}
