use roxmltree::Node;

#[derive(Debug, Clone)]
pub struct CartesianBounds {
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
}

fn extract_double_bound(bounds: &Node, tag_name: &str) -> Option<f64> {
    bounds
        .descendants()
        .find(|n| n.has_tag_name(tag_name))
        .and_then(|n| n.text())
        .and_then(|t| t.parse::<f64>().ok())
}

pub fn cartesian_bounds_from_node(node: &Node) -> CartesianBounds {
    let x_min = extract_double_bound(node, "xMinimum");
    let x_max = extract_double_bound(node, "xMaximum");
    let y_min = extract_double_bound(node, "yMinimum");
    let y_max = extract_double_bound(node, "yMaximum");
    let z_min = extract_double_bound(node, "zMinimum");
    let z_max = extract_double_bound(node, "zMaximum");
    CartesianBounds {
        x_min,
        x_max,
        y_min,
        y_max,
        z_min,
        z_max,
    }
}

#[derive(Debug, Clone)]
pub struct SphericalBounds {
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub elevation_min: Option<f64>,
    pub elevation_max: Option<f64>,
    pub azimuth_start: Option<f64>,
    pub azimuth_end: Option<f64>,
}

pub fn spherical_bounds_from_node(node: &Node) -> SphericalBounds {
    let range_min = extract_double_bound(node, "rangeMinimum");
    let range_max = extract_double_bound(node, "rangeMaximum");
    let elevation_min = extract_double_bound(node, "elevationMinimum");
    let elevation_max = extract_double_bound(node, "elevationMaximum");
    let azimuth_start = extract_double_bound(node, "azimuthStart");
    let azimuth_end = extract_double_bound(node, "azimuthEnd");
    SphericalBounds {
        range_min,
        range_max,
        elevation_min,
        elevation_max,
        azimuth_start,
        azimuth_end,
    }
}
