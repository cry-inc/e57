/// Structure for Cartesian coordinates with an X, Y and Z value.
#[derive(Clone, Debug, PartialEq)]
pub enum CartesianCoordinate {
    /// The Cartesian coordinate is fully valid.
    Valid { x: f64, y: f64, z: f64 },
    /// The Cartesian coordinate only contains a direction vector.
    /// Be careful, the vector might not be normalized!
    Direction { x: f64, y: f64, z: f64 },
    /// The Cartesian coordinate is fully invalid and has no meaning or the point cloud had no cartesian coordinates in general.
    Invalid,
}

/// Spherical coordinates with range, azimuth and elevation.
#[derive(Clone, Debug, PartialEq)]
pub enum SphericalCoordinate {
    /// The spherical coordinate is fully valid.
    Valid {
        range: f64,
        azimuth: f64,
        elevation: f64,
    },
    /// The spherical coordinate only defines direction and has no valid range.
    Direction { azimuth: f64, elevation: f64 },
    /// The spherical coordinate is fully invalid and has no meaning or the point cloud had no spherical coordinates in general.
    Invalid,
}

/// Simple point colors with RGB values between 0 and 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

/// Represents a high level point with its different attributes.
#[derive(Clone, Debug)]
pub struct Point {
    /// Cartesian coordinates.
    /// Might be always invalid if the point cloud does only contain spherical coordinates and the automatic conversion from spherical to Cartesian is disabled.
    pub cartesian: CartesianCoordinate,

    /// Spherical coordinates.
    /// Might be always invalid if the point cloud does only contain Cartesian coordinates.
    pub spherical: SphericalCoordinate,

    /// RGB point colors.
    /// None means the whole point cloud has no colors or the color of this individual point is invalid.
    /// Please check the point cloud properties to understand whether the point cloud in general has color or not.
    pub color: Option<Color>,

    /// Floating point intensity value between 0 and 1.
    /// None means the whole point cloud has no intensity or the intensity of this individual point is invalid.
    /// Please check the point cloud properties to understand whether the point cloud in general has intensity or not.
    pub intensity: Option<f32>,

    /// Row index (Y-axis) to describe point data in a 2D image-like grid.
    /// Default value for point clouds without row index will be -1.
    /// Since this cannot be invalid for individual points, its not an option.
    /// Please check the point cloud properties to understand if the points
    /// have a row index or not.
    pub row: i64,
    /// Column index (X-axis) to describe point data in a 2D image-like grid.
    /// Default value for point clouds without column index will be -1.
    /// Since this cannot be invalid for individual points, its not an option.
    /// Please check the point cloud properties to understand if the points
    /// have a column index or not.
    pub column: i64,
}
