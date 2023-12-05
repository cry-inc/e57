use crate::paged_reader::PagedReader;
use crate::{
    CartesianCoordinate, Color, Error, Point, PointCloud, PointCloudReaderRaw, RawValues,
    RecordName, Result, SphericalCoordinate, Transform, Translation,
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
    transform: bool,
    s2c: bool,
    c2s: bool,
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
            transform: true,
            s2c: true,
            c2s: false,
            i2c: true,
            rotation,
            translation,
            indices,
        })
    }

    /// If enabled, the iterator will automatically convert spherical to Cartesian coordinates.
    /// Will only replace fully invalid Cartesian coordinates and do nothing otherwise.
    /// Default setting is enabled.
    pub fn spherical_to_cartesian(&mut self, enable: bool) {
        self.s2c = enable;
    }

    /// If enabled, the iterator will automatically convert Cartesian to spherical coordinates.
    /// Will only replace fully invalid spherical coordinates and do nothing otherwise.
    /// Default setting is disabled.
    pub fn cartesian_to_spherical(&mut self, enable: bool) {
        self.c2s = enable;
    }

    /// If enabled, the iterator will automatically convert intensity to grey colors.
    /// Will only replace fully invalid color values and do nothing otherwise.
    /// Default setting is enabled.
    pub fn intensity_to_color(&mut self, enable: bool) {
        self.i2c = enable;
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

    fn create_point(&self, values: RawValues) -> Result<Point> {
        let proto = &self.pc.prototype;

        // Cartesian coordinates
        let cartesian_invalid = if let Some(ind) = self.indices.cartesian_invalid {
            values[ind].to_i64(&proto[ind].data_type)?
        } else if self.indices.cartesian.is_some() {
            0
        } else {
            2
        };
        let cartesian = if let Some(ind) = self.indices.cartesian {
            if cartesian_invalid == 0 {
                CartesianCoordinate::Valid {
                    x: values[ind.0].to_f64(&proto[ind.0].data_type)?,
                    y: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                    z: values[ind.2].to_f64(&proto[ind.2].data_type)?,
                }
            } else if cartesian_invalid == 1 {
                CartesianCoordinate::Direction {
                    x: values[ind.0].to_f64(&proto[ind.0].data_type)?,
                    y: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                    z: values[ind.2].to_f64(&proto[ind.2].data_type)?,
                }
            } else if cartesian_invalid == 2 {
                CartesianCoordinate::Invalid
            } else {
                Error::invalid(format!(
                    "Cartesian invalid state contains invalid value: {cartesian_invalid}"
                ))?
            }
        } else {
            CartesianCoordinate::Invalid
        };

        // Spherical coordinates
        let spherical_invalid = if let Some(ind) = self.indices.spherical_invalid {
            values[ind].to_i64(&proto[ind].data_type)?
        } else if self.indices.spherical.is_some() {
            0
        } else {
            2
        };
        let spherical = if let Some(ind) = self.indices.spherical {
            if spherical_invalid == 0 {
                SphericalCoordinate::Valid {
                    range: values[ind.0].to_f64(&proto[ind.0].data_type)?,
                    azimuth: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                    elevation: values[ind.2].to_f64(&proto[ind.2].data_type)?,
                }
            } else if spherical_invalid == 1 {
                SphericalCoordinate::Direction {
                    azimuth: values[ind.1].to_f64(&proto[ind.1].data_type)?,
                    elevation: values[ind.2].to_f64(&proto[ind.2].data_type)?,
                }
            } else if spherical_invalid == 2 {
                SphericalCoordinate::Invalid
            } else {
                Error::invalid(format!(
                    "Spherical invalid state contains invalid value: {spherical_invalid}"
                ))?
            }
        } else {
            SphericalCoordinate::Invalid
        };

        // RGB colors
        let color_invalid = if let Some(ind) = self.indices.color_invalid {
            values[ind].to_i64(&proto[ind].data_type)?
        } else if self.indices.color.is_some() {
            0
        } else {
            1
        };
        let color = if let Some(ind) = self.indices.color {
            if color_invalid == 0 {
                Some(Color {
                    // Use unwrap_or() to make the simple iterator
                    // more robust against weird files that forgot
                    // to add proper min/max values.
                    red: values[ind.0]
                        .to_unit_f32(&proto[ind.0].data_type)
                        .unwrap_or(0.0),
                    green: values[ind.1]
                        .to_unit_f32(&proto[ind.1].data_type)
                        .unwrap_or(0.0),
                    blue: values[ind.2]
                        .to_unit_f32(&proto[ind.2].data_type)
                        .unwrap_or(0.0),
                })
            } else if color_invalid == 1 {
                None
            } else {
                Error::invalid(format!(
                    "Color invalid state contains invalid value: {color_invalid}"
                ))?
            }
        } else {
            None
        };

        // Intensity values
        let intensity_invalid = if let Some(ind) = self.indices.intensity_invalid {
            values[ind].to_i64(&proto[ind].data_type)?
        } else if self.indices.intensity.is_some() {
            0
        } else {
            1
        };
        let intensity = if let Some(ind) = self.indices.intensity {
            if intensity_invalid == 0 {
                Some(
                    // Use unwrap_or() to make the simple iterator
                    // more robust against weird files that forgot
                    // to add proper min/max values.
                    values[ind]
                        .to_unit_f32(&proto[ind].data_type)
                        .unwrap_or(0.0),
                )
            } else if intensity_invalid == 1 {
                None
            } else {
                Error::invalid(format!(
                    "Intensity invalid state contains invalid value: {intensity_invalid}"
                ))?
            }
        } else {
            None
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
            spherical,
            color,
            intensity,
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
        let mut p = match self.get_next_point()? {
            Ok(p) => p,
            Err(err) => return Some(Err(err)),
        };
        if self.s2c {
            convert_to_cartesian(&mut p);
        }
        if self.c2s {
            convert_to_spherical(&mut p);
        }
        if self.i2c {
            convert_intensity(&mut p);
        }
        if self.transform {
            transform_point(&mut p, &self.rotation, &self.translation);
        }
        Some(Ok(p))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.raw_iter.size_hint()
    }
}

fn transform_point(p: &mut Point, rotation: &[f64; 9], translation: &Translation) {
    if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
        let nx = rotation[0] * x + rotation[3] * y + rotation[6] * z;
        let ny = rotation[1] * x + rotation[4] * y + rotation[7] * z;
        let nz = rotation[2] * x + rotation[5] * y + rotation[8] * z;
        p.cartesian = CartesianCoordinate::Valid {
            x: nx + translation.x,
            y: ny + translation.y,
            z: nz + translation.z,
        };
    }
}

fn convert_to_cartesian(p: &mut Point) {
    if let CartesianCoordinate::Valid { .. } = p.cartesian {
        // Abort if there is already a valid coordinate
        return;
    } else if let SphericalCoordinate::Valid {
        range,
        azimuth,
        elevation,
    } = p.spherical
    {
        // Convert valid spherical coordinate to valid Cartesian coordinate
        let cos_ele = f64::cos(elevation);
        p.cartesian = CartesianCoordinate::Valid {
            x: range * cos_ele * f64::cos(azimuth),
            y: range * cos_ele * f64::sin(azimuth),
            z: range * f64::sin(elevation),
        };
        return;
    }

    if let CartesianCoordinate::Direction { .. } = p.cartesian {
        // Do nothing if there is already a valid direction
    } else if let SphericalCoordinate::Direction { azimuth, elevation } = p.spherical {
        // Convert spherical direction coordinate to Cartesian direction
        let cos_ele = f64::cos(elevation);
        p.cartesian = CartesianCoordinate::Direction {
            x: 1.0 * cos_ele * f64::cos(azimuth),
            y: 1.0 * cos_ele * f64::sin(azimuth),
            z: 1.0 * f64::sin(elevation),
        };
    }
}

fn convert_to_spherical(p: &mut Point) {
    if let SphericalCoordinate::Valid { .. } = p.spherical {
        // Abort if there is already a valid coordinate
        return;
    } else if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
        // Convert valid Cartesian coordinate to valid spherical coordinate
        let r = f64::sqrt(x * x + y * y + z * z);
        p.spherical = SphericalCoordinate::Valid {
            range: r,
            azimuth: f64::atan2(y, x),
            elevation: f64::asin(z / r),
        };
        return;
    }

    if let SphericalCoordinate::Direction { .. } = p.spherical {
        // Do nothing if there is already a valid direction
    } else if let CartesianCoordinate::Direction { x, y, z } = p.cartesian {
        // Convert Cartesian direction coordinate to spherical direction
        p.spherical = SphericalCoordinate::Direction {
            azimuth: f64::atan2(y, x),
            elevation: f64::asin(z / f64::sqrt(x * x + y * y + z * z)),
        };
    }
}

fn convert_intensity(p: &mut Point) {
    if p.color.is_some() {
        // Do nothing if there is already valid color
    } else if let Some(intensity) = p.intensity {
        p.color = Some(Color {
            red: intensity,
            green: intensity,
            blue: intensity,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn to_spherical() {
        let mut p = Point {
            cartesian: CartesianCoordinate::Valid {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
            spherical: SphericalCoordinate::Invalid,
            color: None,
            intensity: None,
            row: -1,
            column: -1,
        };
        convert_to_spherical(&mut p);
        assert_eq!(
            p.spherical,
            SphericalCoordinate::Valid {
                range: f64::sqrt(2.0),
                azimuth: PI / 4.0,
                elevation: 0.0
            }
        );
    }

    #[test]
    fn to_cartesian() {
        let mut p = Point {
            cartesian: CartesianCoordinate::Invalid,
            spherical: SphericalCoordinate::Valid {
                range: 10.0,
                azimuth: 0.0,
                elevation: PI / 2.0,
            },
            color: None,
            intensity: None,
            row: -1,
            column: -1,
        };
        convert_to_cartesian(&mut p);
        if let CartesianCoordinate::Valid { x, y, z } = p.cartesian {
            assert!(x.abs() < 0.000001);
            assert_eq!(y, 0.0);
            assert_eq!(z, 10.0);
        } else {
            assert!(false)
        }
    }

    #[test]
    fn roundtrip_conversion() {
        let cartesian = [1.0, 2.0, 3.0];
        let mut point = Point {
            cartesian: CartesianCoordinate::Valid {
                x: cartesian[0],
                y: cartesian[1],
                z: cartesian[2],
            },
            spherical: SphericalCoordinate::Invalid,
            color: None,
            intensity: None,
            row: -1,
            column: -1,
        };
        convert_to_spherical(&mut point);
        point.cartesian = CartesianCoordinate::Invalid;
        convert_to_cartesian(&mut point);
        if let CartesianCoordinate::Valid { x, y, z } = point.cartesian {
            assert!((x - cartesian[0]).abs() < 0.00001);
            assert!((y - cartesian[1]).abs() < 0.00001);
            assert!((z - cartesian[2]).abs() < 0.00001);
        } else {
            assert!(false);
        }
    }
}
