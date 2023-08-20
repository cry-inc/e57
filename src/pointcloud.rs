use crate::error::Converter;
use crate::xml;
use crate::{
    CartesianBounds, ColorLimits, DateTime, Error, IndexBounds, IntensityLimits, Record,
    RecordDataType, RecordName, Result, SphericalBounds, Transform,
};
use roxmltree::{Document, Node};

/// Descriptor with metadata for a single point cloud.
///
/// This struct does not contain any actual point data,
/// it just describes the properties and attributes of a point cloud.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PointCloud {
    /// Globally unique identifier for the point cloud.
    pub guid: String,
    /// Physical file offset of the start of the associated binary section.
    pub file_offset: u64,
    /// Number of points in the point cloud.
    pub records: u64,
    /// List of point attributes that exist for this point cloud.
    pub prototype: Vec<Record>,

    /// Optional user-defined name for the point cloud.
    pub name: Option<String>,
    /// Optional user-defined description of the point cloud.
    pub description: Option<String>,
    /// Optional Cartesian bounds for the point cloud.
    pub cartesian_bounds: Option<CartesianBounds>,
    /// Optional spherical bounds for the point cloud.
    pub spherical_bounds: Option<SphericalBounds>,
    /// Optional index bounds (row, column, return values) for the point cloud.
    pub index_bounds: Option<IndexBounds>,
    /// Optional intensity limits for the point cloud.
    pub intensity_limits: Option<IntensityLimits>,
    /// Optional color limits for the point cloud.
    pub color_limits: Option<ColorLimits>,
    /// Optional transformation to convert data from the local point cloud coordinates to the file-level coordinate system.
    pub transform: Option<Transform>,
    /// Optional start date and time when the point cloud was captured with a scanning device.
    pub acquisition_start: Option<DateTime>,
    /// Optional end date and time when the point cloud was captured with a scanning device.
    pub acquisition_end: Option<DateTime>,
    /// Optional name of the manufacturer for the sensor used to capture the point cloud.
    pub sensor_vendor: Option<String>,
    /// Optional model name of the sensor used for capturing.
    pub sensor_model: Option<String>,
    /// Optional serial number of the sensor used for capturing.
    pub sensor_serial: Option<String>,
    /// Optional version identifier for the sensor hardware used for capturing.
    pub sensor_hw_version: Option<String>,
    /// Optional version identifier for the sensor software used for capturing.
    pub sensor_sw_version: Option<String>,
    /// Optional version identifier for the sensor firmware used for capturing.
    pub sensor_fw_version: Option<String>,
    /// Optional ambient temperature in degrees Celsius, measured at the sensor at the time of capturing.
    pub temperature: Option<f64>,
    /// Optional percentage of relative humidity between 0 and 100, measured at the sensor at the time of capturing.
    pub humidity: Option<f64>,
    /// Optional atmospheric pressure in Pascals, measured at the sensor at the time of capturing.
    pub atmospheric_pressure: Option<f64>,
}

impl PointCloud {
    pub(crate) fn vec_from_document(document: &Document) -> Result<Vec<Self>> {
        let data3d_node = document
            .descendants()
            .find(|n| n.has_tag_name("data3D"))
            .invalid_err("Cannot find 'data3D' tag in XML document")?;

        let mut pointclouds = Vec::new();
        for n in data3d_node.children() {
            if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
                let pointcloud = Self::from_node(&n)?;
                pointclouds.push(pointcloud);
            }
        }
        Ok(pointclouds)
    }

    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        let guid = xml::req_string(node, "guid")?;
        let name = xml::opt_string(node, "name")?;
        let description = xml::opt_string(node, "description")?;
        let sensor_model = xml::opt_string(node, "sensorModel")?;
        let sensor_vendor = xml::opt_string(node, "sensorVendor")?;
        let sensor_serial = xml::opt_string(node, "sensorSerialNumber")?;
        let sensor_hw_version = xml::opt_string(node, "sensorHardwareVersion")?;
        let sensor_sw_version = xml::opt_string(node, "sensorSoftwareVersion")?;
        let sensor_fw_version = xml::opt_string(node, "sensorFirmwareVersion")?;
        let temperature = xml::opt_f64(node, "temperature")?;
        let humidity = xml::opt_f64(node, "relativeHumidity")?;
        let atmospheric_pressure = xml::opt_f64(node, "atmosphericPressure")?;
        let acquisition_start = xml::opt_date_time(node, "acquisitionStart")?;
        let acquisition_end = xml::opt_date_time(node, "acquisitionEnd")?;
        let transform = xml::opt_transform(node, "pose")?;
        let cartesian_bounds = node.children().find(|n| n.has_tag_name("cartesianBounds"));
        let spherical_bounds = node.children().find(|n| n.has_tag_name("sphericalBounds"));
        let index_bounds = node.children().find(|n| n.has_tag_name("indexBounds"));
        let intensity_limits = node.children().find(|n| n.has_tag_name("colorLimits"));
        let color_limits = node.children().find(|n| n.has_tag_name("colorLimits"));

        let points_tag = node
            .children()
            .find(|n| n.has_tag_name("points") && n.attribute("type") == Some("CompressedVector"))
            .invalid_err("Cannot find 'points' tag inside 'data3D' child")?;
        let file_offset = points_tag
            .attribute("fileOffset")
            .invalid_err("Cannot find 'fileOffset' attribute in 'points' tag")?
            .parse::<u64>()
            .invalid_err("Cannot parse 'fileOffset' attribute value as u64")?;
        let records = points_tag
            .attribute("recordCount")
            .invalid_err("Cannot find 'recordCount' attribute in 'points' tag")?
            .parse::<u64>()
            .invalid_err("Cannot parse 'recordCount' attribute value as u64")?;
        let prototype_tag = points_tag
            .children()
            .find(|n| n.has_tag_name("prototype") && n.attribute("type") == Some("Structure"))
            .invalid_err("Cannot find 'prototype' child in 'points' tag")?;
        let mut prototype = Vec::new();
        for n in prototype_tag.children() {
            if !n.is_element() {
                continue;
            }
            let ns = n.lookup_prefix(n.tag_name().namespace().unwrap_or_default());
            let tag = n.tag_name().name();
            let name = RecordName::from_namespace_and_tag_name(ns, tag)?;
            let data_type = RecordDataType::from_node(&n)?;
            prototype.push(Record { name, data_type });
        }

        Ok(Self {
            guid,
            name,
            file_offset,
            records,
            prototype,
            cartesian_bounds: if let Some(node) = cartesian_bounds {
                Some(CartesianBounds::from_node(&node)?)
            } else {
                None
            },
            spherical_bounds: if let Some(node) = spherical_bounds {
                Some(SphericalBounds::from_node(&node)?)
            } else {
                None
            },
            index_bounds: if let Some(node) = index_bounds {
                Some(IndexBounds::from_node(&node)?)
            } else {
                None
            },
            intensity_limits: if let Some(node) = intensity_limits {
                Some(IntensityLimits::from_node(&node)?)
            } else {
                None
            },
            color_limits: if let Some(node) = color_limits {
                Some(ColorLimits::from_node(&node)?)
            } else {
                None
            },
            transform,
            description,
            acquisition_start,
            acquisition_end,
            sensor_vendor,
            sensor_model,
            sensor_serial,
            sensor_hw_version,
            sensor_sw_version,
            sensor_fw_version,
            temperature,
            humidity,
            atmospheric_pressure,
        })
    }

    pub fn xml_string(&self) -> Result<String> {
        let mut xml = String::new();
        xml += "<vectorChild type=\"Structure\">\n";
        if self.guid.is_empty() {
            Error::invalid("Empty point cloud GUID is not allowed")?
        }
        xml += &xml::gen_string("guid", &self.guid);

        if let Some(bounds) = &self.cartesian_bounds {
            xml += &bounds.xml_string();
        }
        if let Some(bounds) = &self.spherical_bounds {
            xml += &bounds.xml_string();
        }
        if let Some(bounds) = &self.index_bounds {
            xml += &bounds.xml_string();
        }

        if let Some(limits) = &self.color_limits {
            xml += &limits.xml_string();
        }
        if let Some(limits) = &self.intensity_limits {
            xml += &limits.xml_string();
        }

        if let Some(name) = &self.name {
            xml += &xml::gen_string("name", name);
        }
        if let Some(desc) = &self.description {
            xml += &xml::gen_string("description", desc);
        }
        if let Some(sensor_vendor) = &self.sensor_vendor {
            xml += &xml::gen_string("sensorVendor", sensor_vendor);
        }
        if let Some(sensor_model) = &self.sensor_model {
            xml += &xml::gen_string("sensorModel", sensor_model);
        }
        if let Some(sensor_serial) = &self.sensor_serial {
            xml += &xml::gen_string("sensorSerialNumber", sensor_serial);
        }
        if let Some(sensor_sw_version) = &self.sensor_sw_version {
            xml += &xml::gen_string("sensorSoftwareVersion", sensor_sw_version);
        }
        if let Some(sensor_fw_version) = &self.sensor_fw_version {
            xml += &xml::gen_string("sensorFirmwareVersion", sensor_fw_version);
        }
        if let Some(sensor_hw_version) = &self.sensor_hw_version {
            xml += &xml::gen_string("sensorHardwareVersion", sensor_hw_version);
        }

        if let Some(transform) = &self.transform {
            xml += &transform.xml_string("pose");
        }
        if let Some(aq_start) = &self.acquisition_start {
            xml += &aq_start.xml_string("acquisitionStart");
        }
        if let Some(aq_end) = &self.acquisition_end {
            xml += &aq_end.xml_string("acquisitionEnd");
        }

        if let Some(temperature) = self.temperature {
            xml += &xml::gen_float("temperature", temperature);
        }
        if let Some(humidity) = self.humidity {
            xml += &xml::gen_float("relativeHumidity", humidity);
        }
        if let Some(pressure) = self.atmospheric_pressure {
            xml += &xml::gen_float("atmosphericPressure", pressure);
        }

        xml += &format!(
            "<points type=\"CompressedVector\" fileOffset=\"{}\" recordCount=\"{}\">\n",
            self.file_offset, self.records
        );
        xml += "<prototype type=\"Structure\">\n";
        for record in &self.prototype {
            xml += &record.xml_string();
        }
        xml += "</prototype>\n";
        xml += "</points>\n";

        xml += "</vectorChild>\n";
        Ok(xml)
    }
}
