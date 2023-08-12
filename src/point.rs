use crate::error::Converter;
use crate::{RawValues, Record, RecordName, Result};
use std::collections::HashMap;

/// Simple structure for cartesian coordinates with an X, Y and Z value.
#[derive(Clone, Debug, Default)]
pub struct CartesianCoordinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Simple spherical coordinates with range, azimuth and elevation.
#[derive(Clone, Debug, Default)]
pub struct SphericalCoordinate {
    pub range: f64,
    pub azimuth: f64,
    pub elevation: f64,
}

/// Simple point colors with RGB values between 0 and 1.
#[derive(Clone, Debug, Default)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

/// Return index and count. Only used for multi-return sensors.
#[derive(Clone, Debug)]
pub struct Return {
    pub index: i64,
    pub count: i64,
}

/// Represents a high level point with its different attributes.
#[derive(Clone, Debug)]
pub struct Point {
    /// Cartesian XYZ coordinates.
    pub cartesian: CartesianCoordinate,
    /// Invalid states of the Cartesian coordinates.
    /// 0 means valid, 1: means its a direction vector, 2 means fully invalid.
    pub cartesian_invalid: u8,

    /// Spherical coordinates with range, azimuth and elevation.
    pub spherical: SphericalCoordinate,
    /// Invalid states of the spherical coordinates.
    /// 0 means valid, 1: means range is not meaningful, 2 means fully invalid.
    pub spherical_invalid: u8,

    /// RGB point colors.
    pub color: Color,
    /// A value of zero means the color is valid, 1 means invalid.
    pub color_invalid: u8,

    /// Floating point intensity value between 0 and 1.
    pub intensity: f32,
    /// A value of zero means the intensity is valid, 1 means invalid.
    pub intensity_invalid: u8,

    /// Row index (Y-axis) to describe point data in a 2D image-like grid.
    /// Default value for point clouds without row index will be -1.
    pub row: i64,
    /// Column index (X-axis) to describe point data in a 2D image-like grid.
    /// Default value for point clouds without column index will be -1.
    pub column: i64,
}

impl Point {
    pub(crate) fn from_values(values: RawValues, prototype: &[Record]) -> Result<Self> {
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
