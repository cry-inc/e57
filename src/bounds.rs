use crate::xml::optional_double;
use crate::xml::optional_integer;
use crate::Result;
use roxmltree::Node;

/// Optional minimum and maximum values for Cartesian X, Y and Z coordinates.
#[derive(Clone, Debug, Default)]
pub struct CartesianBounds {
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
}

impl CartesianBounds {
    pub(crate) fn from_node(node: &Node) -> Result<CartesianBounds> {
        let x_min = optional_double(node, "xMinimum")?;
        let x_max = optional_double(node, "xMaximum")?;
        let y_min = optional_double(node, "yMinimum")?;
        let y_max = optional_double(node, "yMaximum")?;
        let z_min = optional_double(node, "zMinimum")?;
        let z_max = optional_double(node, "zMaximum")?;
        Ok(CartesianBounds {
            x_min,
            x_max,
            y_min,
            y_max,
            z_min,
            z_max,
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::from("<cartesianBounds type=\"Structure\">");
        if let Some(min) = self.x_min {
            xml += &format!("<xMinimum type=\"Float\">{min}</xMinimum>");
        }
        if let Some(max) = self.x_max {
            xml += &format!("<xMaximum type=\"Float\">{max}</xMaximum>");
        }
        if let Some(min) = self.y_min {
            xml += &format!("<yMinimum type=\"Float\">{min}</yMinimum>");
        }
        if let Some(max) = self.y_max {
            xml += &format!("<yMaximum type=\"Float\">{max}</yMaximum>");
        }
        if let Some(min) = self.z_min {
            xml += &format!("<zMinimum type=\"Float\">{min}</zMinimum>");
        }
        if let Some(max) = self.z_max {
            xml += &format!("<zMaximum type=\"Float\">{max}</zMaximum>");
        }
        xml += "</cartesianBounds>";
        xml
    }
}

/// Optional minimum and maximum values for spherical coordinates.
#[derive(Clone, Debug, Default)]
pub struct SphericalBounds {
    pub range_min: Option<f64>,
    pub range_max: Option<f64>,
    pub elevation_min: Option<f64>,
    pub elevation_max: Option<f64>,
    pub azimuth_start: Option<f64>,
    pub azimuth_end: Option<f64>,
}

impl SphericalBounds {
    pub(crate) fn from_node(node: &Node) -> Result<SphericalBounds> {
        let range_min = optional_double(node, "rangeMinimum")?;
        let range_max = optional_double(node, "rangeMaximum")?;
        let elevation_min = optional_double(node, "elevationMinimum")?;
        let elevation_max = optional_double(node, "elevationMaximum")?;
        let azimuth_start = optional_double(node, "azimuthStart")?;
        let azimuth_end = optional_double(node, "azimuthEnd")?;
        Ok(SphericalBounds {
            range_min,
            range_max,
            elevation_min,
            elevation_max,
            azimuth_start,
            azimuth_end,
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::from("<sphericalBounds type=\"Structure\">");
        if let Some(min) = self.azimuth_start {
            xml += &format!("<azimuthStart type=\"Float\">{min}</azimuthStart>");
        }
        if let Some(max) = self.azimuth_end {
            xml += &format!("<azimuthEnd type=\"Float\">{max}</azimuthEnd>");
        }
        if let Some(min) = self.elevation_min {
            xml += &format!("<elevationMinimum type=\"Float\">{min}</elevationMinimum>");
        }
        if let Some(max) = self.elevation_max {
            xml += &format!("<elevationMaximum type=\"Float\">{max}</elevationMaximum>");
        }
        if let Some(min) = self.range_min {
            xml += &format!("<rangeMinimum type=\"Float\">{min}</rangeMinimum>");
        }
        if let Some(max) = self.range_max {
            xml += &format!("<rangeMaximum type=\"Float\">{max}</rangeMaximum>");
        }
        xml += "</sphericalBounds>";
        xml
    }
}

/// Optional minimum and maximum values for the row, column and return indices.
#[derive(Clone, Debug, Default)]
pub struct IndexBounds {
    pub row_min: Option<i64>,
    pub row_max: Option<i64>,
    pub column_min: Option<i64>,
    pub column_max: Option<i64>,
    pub return_min: Option<i64>,
    pub return_max: Option<i64>,
}

impl IndexBounds {
    pub(crate) fn from_node(node: &Node) -> Result<IndexBounds> {
        let row_min = optional_integer(node, "rowMinimum")?;
        let row_max = optional_integer(node, "rowMaximum")?;
        let column_min = optional_integer(node, "columnMinimum")?;
        let column_max = optional_integer(node, "columnMaximum")?;
        let return_min = optional_integer(node, "returnMinimum")?;
        let return_max = optional_integer(node, "returnMaximum")?;
        Ok(IndexBounds {
            row_min,
            row_max,
            column_min,
            column_max,
            return_min,
            return_max,
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::from("<indexBounds type=\"Structure\">");
        if let Some(min) = self.row_min {
            xml += &format!("<rowMinimum type=\"Integer\">{min}</rowMinimum>");
        }
        if let Some(max) = self.row_max {
            xml += &format!("<rowMaximum type=\"Integer\">{max}</rowMaximum>");
        }
        if let Some(min) = self.column_min {
            xml += &format!("<columnMinimum type=\"Integer\">{min}</columnMinimum>");
        }
        if let Some(max) = self.column_max {
            xml += &format!("<columnMaximum type=\"Integer\">{max}</columnMaximum>");
        }
        if let Some(min) = self.return_min {
            xml += &format!("<returnMinimum type=\"Integer\">{min}</returnMinimum>");
        }
        if let Some(max) = self.return_max {
            xml += &format!("<returnMaximum type=\"Integer\">{max}</returnMaximum>");
        }
        xml += "</indexBounds>";
        xml
    }
}
