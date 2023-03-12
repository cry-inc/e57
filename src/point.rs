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

/// Represents a single individual point with all its different attributes.
#[derive(Clone, Debug, Default)]
pub struct Point {
    pub cartesian: Option<CartesianCoordinate>,
    pub spherical: Option<SphericalCoordinate>,
    pub color: Option<Color>,
    pub ret: Option<Return>,
    pub row: Option<i64>,
    pub column: Option<i64>,
    pub time: Option<f64>,
    /// Intensity value between 0 and 1.
    pub intensity: Option<f32>,
    pub cartesian_invalid: Option<u8>,
    pub spherical_invalid: Option<u8>,
    pub time_invalid: Option<u8>,
    pub intensity_invalid: Option<u8>,
    pub color_invalid: Option<u8>,
}
