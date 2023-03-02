use roxmltree::Node;
use std::str::FromStr;

fn extract_bound<T: FromStr>(bounds: &Node, tag_name: &str) -> Option<T> {
    bounds
        .descendants()
        .find(|n| n.has_tag_name(tag_name))
        .and_then(|n| n.text())
        .and_then(|t| t.parse::<T>().ok())
}

/// Optional minimum and maximum values for Cartesian X, Y and Z coordinates.
#[derive(Clone, Debug)]
pub struct CartesianBounds {
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
}

pub fn cartesian_bounds_from_node(node: &Node) -> CartesianBounds {
    let x_min = extract_bound(node, "xMinimum");
    let x_max = extract_bound(node, "xMaximum");
    let y_min = extract_bound(node, "yMinimum");
    let y_max = extract_bound(node, "yMaximum");
    let z_min = extract_bound(node, "zMinimum");
    let z_max = extract_bound(node, "zMaximum");
    CartesianBounds {
        x_min,
        x_max,
        y_min,
        y_max,
        z_min,
        z_max,
    }
}

/// Optional minimum and maximum values for spherical coordinates.
#[derive(Clone, Debug)]
pub struct SphericalBounds {
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub elevation_min: Option<f64>,
    pub elevation_max: Option<f64>,
    pub azimuth_start: Option<f64>,
    pub azimuth_end: Option<f64>,
}

pub fn spherical_bounds_from_node(node: &Node) -> SphericalBounds {
    let range_min = extract_bound(node, "rangeMinimum");
    let range_max = extract_bound(node, "rangeMaximum");
    let elevation_min = extract_bound(node, "elevationMinimum");
    let elevation_max = extract_bound(node, "elevationMaximum");
    let azimuth_start = extract_bound(node, "azimuthStart");
    let azimuth_end = extract_bound(node, "azimuthEnd");
    SphericalBounds {
        range_min,
        range_max,
        elevation_min,
        elevation_max,
        azimuth_start,
        azimuth_end,
    }
}

/// Optional minimum and maximum values for the row, column and return indices.
#[derive(Clone, Debug)]
pub struct IndexBounds {
    pub row_min: Option<i64>,
    pub row_max: Option<i64>,
    pub column_min: Option<i64>,
    pub column_max: Option<i64>,
    pub return_min: Option<i64>,
    pub return_max: Option<i64>,
}

pub fn index_bounds_from_node(node: &Node) -> IndexBounds {
    let row_min = extract_bound(node, "rowMinimum");
    let row_max = extract_bound(node, "rowMaximum");
    let column_min = extract_bound(node, "columnMinimum");
    let column_max = extract_bound(node, "columnMaximum");
    let return_min = extract_bound(node, "returnMinimum");
    let return_max = extract_bound(node, "returnMaximum");
    IndexBounds {
        row_min,
        row_max,
        column_min,
        column_max,
        return_min,
        return_max,
    }
}
