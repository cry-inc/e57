use crate::error::Converter;
use crate::{RawValues, Record, RecordName, Result};
use std::collections::HashMap;

/// Simple structure for cartesian coordinates with an X, Y and Z value.
#[derive(Clone, Debug)]
pub struct CartesianCoordinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Simple spherical coordinates with range, azimuth and elevation.
#[derive(Clone, Debug)]
pub struct SphericalCoordinate {
    pub range: f64,
    pub azimuth: f64,
    pub elevation: f64,
}

/// Simple point colors with RGB values between 0 and 1.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug, Default)]
pub struct Point {
    /// Cartesian XYZ coordinates.
    pub cartesian: Option<CartesianCoordinate>,
    /// Invalid states of the Cartesian coordinates.
    /// 0 means valid, 1: means its a direction vector, 2 means fully invalid.
    pub cartesian_invalid: Option<u8>,

    /// Spherical coordinates with range, azimuth and elevation.
    pub spherical: Option<SphericalCoordinate>,
    /// Invalid states of the spherical coordinates.
    /// 0 means valid, 1: means range is not meaningful, 2 means fully invalid.
    pub spherical_invalid: Option<u8>,

    /// RGB point colors.
    pub color: Option<Color>,
    /// A value of zero means the color is valid, 1 means invalid.
    pub color_invalid: Option<u8>,

    /// Floating point intensity value between 0 and 1.
    pub intensity: Option<f32>,
    /// A value of zero means the intensity is valid, 1 means invalid.
    pub intensity_invalid: Option<u8>,

    /// Point return values with index and count.
    pub ret: Option<Return>,

    /// Row index (Y-axis) to describe point data in a 2D image-like grid.
    pub row: Option<i64>,
    /// Column index (X-axis) to describe point data in a 2D image-like grid.
    pub column: Option<i64>,

    /// Recording/capture time of the point in seconds relative to scan capture start.
    pub time: Option<f64>,
    /// A value of zero means the time is valid, 1 means invalid.
    pub time_invalid: Option<u8>,
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

        let mut sp = Point::default();
        if let (Some((xt, xv)), Some((yt, yv)), Some((zt, zv))) = (
            data.get(&RecordName::CartesianX),
            data.get(&RecordName::CartesianY),
            data.get(&RecordName::CartesianZ),
        ) {
            sp.cartesian = Some(CartesianCoordinate {
                x: xv.to_f64(xt)?,
                y: yv.to_f64(yt)?,
                z: zv.to_f64(zt)?,
            });
        }
        if let Some((cit, civ)) = data.get(&RecordName::CartesianInvalidState) {
            sp.cartesian_invalid = Some(civ.to_u8(cit)?);
        }
        if let (Some((at, av)), Some((et, ev)), Some((rt, rv))) = (
            data.get(&RecordName::SphericalAzimuth),
            data.get(&RecordName::SphericalElevation),
            data.get(&RecordName::SphericalRange),
        ) {
            sp.spherical = Some(SphericalCoordinate {
                azimuth: av.to_f64(at)?,
                elevation: ev.to_f64(et)?,
                range: rv.to_f64(rt)?,
            });
        }
        if let Some((sit, siv)) = data.get(&RecordName::SphericalInvalidState) {
            sp.spherical_invalid = Some(siv.to_u8(sit)?);
        }
        if let (Some((rt, rv)), Some((gt, gv)), Some((bt, bv))) = (
            data.get(&RecordName::ColorRed),
            data.get(&RecordName::ColorGreen),
            data.get(&RecordName::ColorBlue),
        ) {
            sp.color = Some(Color {
                red: rv.to_unit_f32(rt)?,
                green: gv.to_unit_f32(gt)?,
                blue: bv.to_unit_f32(bt)?,
            });
        }
        if let Some((cit, civ)) = data.get(&RecordName::IsColorInvalid) {
            sp.color_invalid = Some(civ.to_u8(cit)?);
        }
        if let Some((cit, civ)) = data.get(&RecordName::IsColorInvalid) {
            sp.color_invalid = Some(civ.to_u8(cit)?);
        }
        if let Some((it, iv)) = data.get(&RecordName::Intensity) {
            sp.intensity = Some(iv.to_unit_f32(it)?);
        }
        if let Some((iit, iiv)) = data.get(&RecordName::IsIntensityInvalid) {
            sp.intensity_invalid = Some(iiv.to_u8(iit)?);
        }
        if let (Some((rit, riv)), Some((rct, rcv))) = (
            data.get(&RecordName::ReturnIndex),
            data.get(&RecordName::ReturnCount),
        ) {
            sp.ret = Some(Return {
                index: riv.to_i64(rit)?,
                count: rcv.to_i64(rct)?,
            });
        }
        if let Some((rt, rv)) = data.get(&RecordName::RowIndex) {
            sp.row = Some(rv.to_i64(rt)?);
        }
        if let Some((ct, cv)) = data.get(&RecordName::ColumnIndex) {
            sp.column = Some(cv.to_i64(ct)?);
        }
        if let Some((tt, tv)) = data.get(&RecordName::TimeStamp) {
            sp.time = Some(tv.to_f64(tt)?);
        }
        if let Some((tit, tiv)) = data.get(&RecordName::IsTimeStampInvalid) {
            sp.time_invalid = Some(tiv.to_u8(tit)?);
        }
        Ok(sp)
    }
}
