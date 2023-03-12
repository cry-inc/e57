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
