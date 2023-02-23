/// Simple structure for cartesian coordinates with an X, Y and Z value.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CartesianCoodinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Simple spherical coordinates with an X, Y and Z value.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SphericalCoodinate {
    pub range: f64,
    pub azimuth: f64,
    pub elevation: f64,
}

/// Simple point colors with RGB values between 0 and 1.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Return {
    pub index: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Point {
    pub cartesian: Option<CartesianCoodinate>,
    pub spherical: Option<SphericalCoodinate>,
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
