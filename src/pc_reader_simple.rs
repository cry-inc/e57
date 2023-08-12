use crate::error::Converter;
use crate::paged_reader::PagedReader;
use crate::{
    CartesianCoordinate, Color, Point, PointCloud, PointCloudReaderRaw, RawValues, Record,
    RecordName, Result, SphericalCoordinate, Transform, Translation,
};
use std::collections::HashMap;
use std::io::{Read, Seek};

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
            transform: true,
            s2c: true,
            i2c: true,
            rotation,
            translation,
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
            Ok(p) => Some(Self::create_point(p, &self.pc.prototype)),
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

    fn create_point(values: RawValues, prototype: &[Record]) -> Result<Point> {
        let mut data = HashMap::new();
        for (i, p) in prototype.iter().enumerate() {
            let value = values
                .get(i)
                .invalid_err("Cannot find value defined by prototype")?;
            data.insert(p.name.clone(), (p.data_type.clone(), value.clone()));
        }

        let (cartesian, has_cartesian) = if let (Some((xt, xv)), Some((yt, yv)), Some((zt, zv))) = (
            data.get(&RecordName::CartesianX),
            data.get(&RecordName::CartesianY),
            data.get(&RecordName::CartesianZ),
        ) {
            (
                CartesianCoordinate {
                    x: xv.to_f64(xt)?,
                    y: yv.to_f64(yt)?,
                    z: zv.to_f64(zt)?,
                },
                true,
            )
        } else {
            (CartesianCoordinate::default(), false)
        };
        let cartesian_invalid =
            if let Some((cit, civ)) = data.get(&RecordName::CartesianInvalidState) {
                civ.to_u8(cit)?
            } else if has_cartesian {
                0
            } else {
                2
            };
        let (spherical, has_spherical) = if let (Some((at, av)), Some((et, ev)), Some((rt, rv))) = (
            data.get(&RecordName::SphericalAzimuth),
            data.get(&RecordName::SphericalElevation),
            data.get(&RecordName::SphericalRange),
        ) {
            (
                SphericalCoordinate {
                    azimuth: av.to_f64(at)?,
                    elevation: ev.to_f64(et)?,
                    range: rv.to_f64(rt)?,
                },
                true,
            )
        } else {
            (SphericalCoordinate::default(), false)
        };
        let spherical_invalid =
            if let Some((sit, siv)) = data.get(&RecordName::SphericalInvalidState) {
                siv.to_u8(sit)?
            } else if has_spherical {
                0
            } else {
                2
            };

        let (color, has_color) = if let (Some((rt, rv)), Some((gt, gv)), Some((bt, bv))) = (
            data.get(&RecordName::ColorRed),
            data.get(&RecordName::ColorGreen),
            data.get(&RecordName::ColorBlue),
        ) {
            (
                Color {
                    red: rv.to_unit_f32(rt)?,
                    green: gv.to_unit_f32(gt)?,
                    blue: bv.to_unit_f32(bt)?,
                },
                true,
            )
        } else {
            (Color::default(), false)
        };
        let color_invalid = if let Some((cit, civ)) = data.get(&RecordName::IsColorInvalid) {
            civ.to_u8(cit)?
        } else if has_color {
            0
        } else {
            1
        };
        let (intensity, has_intensity) = if let Some((it, iv)) = data.get(&RecordName::Intensity) {
            (iv.to_unit_f32(it)?, true)
        } else {
            (0.0, false)
        };
        let intensity_invalid = if let Some((iit, iiv)) = data.get(&RecordName::IsIntensityInvalid)
        {
            iiv.to_u8(iit)?
        } else if has_intensity {
            0
        } else {
            1
        };
        let row = if let Some((rt, rv)) = data.get(&RecordName::RowIndex) {
            rv.to_i64(rt)?
        } else {
            -1
        };
        let column = if let Some((ct, cv)) = data.get(&RecordName::ColumnIndex) {
            cv.to_i64(ct)?
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
