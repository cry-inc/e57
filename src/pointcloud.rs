use crate::date_time::serialize_date_time;
use crate::error::Converter;
use crate::transform::serialize_transform;
use crate::xml::{
    generate_f64_xml, generate_string_xml, optional_date_time, optional_double, optional_string,
    optional_transform, required_string,
};
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

pub fn pointclouds_from_document(document: &Document) -> Result<Vec<PointCloud>> {
    let data3d_node = document
        .descendants()
        .find(|n| n.has_tag_name("data3D"))
        .invalid_err("Cannot find 'data3D' tag in XML document")?;

    let mut pointclouds = Vec::new();
    for n in data3d_node.children() {
        if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
            let pointcloud = extract_pointcloud(&n)?;
            pointclouds.push(pointcloud);
        }
    }
    Ok(pointclouds)
}

fn extract_pointcloud(node: &Node) -> Result<PointCloud> {
    let guid = required_string(node, "guid")?;
    let name = optional_string(node, "name")?;
    let description = optional_string(node, "description")?;
    let sensor_model = optional_string(node, "sensorModel")?;
    let sensor_vendor = optional_string(node, "sensorVendor")?;
    let sensor_serial = optional_string(node, "sensorSerialNumber")?;
    let sensor_hw_version = optional_string(node, "sensorHardwareVersion")?;
    let sensor_sw_version = optional_string(node, "sensorSoftwareVersion")?;
    let sensor_fw_version = optional_string(node, "sensorFirmwareVersion")?;
    let temperature = optional_double(node, "temperature")?;
    let humidity = optional_double(node, "relativeHumidity")?;
    let atmospheric_pressure = optional_double(node, "atmosphericPressure")?;
    let acquisition_start = optional_date_time(node, "acquisitionStart")?;
    let acquisition_end = optional_date_time(node, "acquisitionEnd")?;
    let transform = optional_transform(node, "pose")?;
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
        if n.is_element() {
            let tag_name = n.tag_name().name();
            let name = RecordName::from_tag_name(tag_name)?;
            let data_type = RecordDataType::from_node(&n)?;
            prototype.push(Record { name, data_type });
        }
    }

    Ok(PointCloud {
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

pub fn serialize_pointcloud(pointcloud: &PointCloud) -> Result<String> {
    let mut xml = String::new();
    xml += "<vectorChild type=\"Structure\">\n";
    if pointcloud.guid.is_empty() {
        Error::invalid("Empty point cloud GUID is not allowed")?
    }
    xml += &generate_string_xml("guid", &pointcloud.guid);

    if let Some(bounds) = &pointcloud.cartesian_bounds {
        xml += &bounds.xml_string();
    }
    if let Some(bounds) = &pointcloud.spherical_bounds {
        xml += &bounds.xml_string();
    }
    if let Some(bounds) = &pointcloud.index_bounds {
        xml += &bounds.xml_string();
    }

    if let Some(limits) = &pointcloud.color_limits {
        xml += &limits.xml_string();
    }
    if let Some(limits) = &pointcloud.intensity_limits {
        xml += &limits.xml_string();
    }

    if let Some(name) = &pointcloud.name {
        xml += &generate_string_xml("name", name);
    }
    if let Some(desc) = &pointcloud.description {
        xml += &generate_string_xml("description", desc);
    }
    if let Some(sensor_vendor) = &pointcloud.sensor_vendor {
        xml += &generate_string_xml("sensorVendor", sensor_vendor);
    }
    if let Some(sensor_model) = &pointcloud.sensor_model {
        xml += &generate_string_xml("sensorModel", sensor_model);
    }
    if let Some(sensor_serial) = &pointcloud.sensor_serial {
        xml += &generate_string_xml("sensorSerialNumber", sensor_serial);
    }
    if let Some(sensor_sw_version) = &pointcloud.sensor_sw_version {
        xml += &generate_string_xml("sensorSoftwareVersion", sensor_sw_version);
    }
    if let Some(sensor_fw_version) = &pointcloud.sensor_fw_version {
        xml += &generate_string_xml("sensorFirmwareVersion", sensor_fw_version);
    }
    if let Some(sensor_hw_version) = &pointcloud.sensor_hw_version {
        xml += &generate_string_xml("sensorHardwareVersion", sensor_hw_version);
    }

    if let Some(transform) = &pointcloud.transform {
        xml += &serialize_transform(transform, "pose");
    }
    if let Some(aq_start) = &pointcloud.acquisition_start {
        xml += &serialize_date_time(aq_start, "acquisitionStart");
    }
    if let Some(aq_end) = &pointcloud.acquisition_end {
        xml += &serialize_date_time(aq_end, "acquisitionEnd");
    }

    if let Some(temperature) = pointcloud.temperature {
        xml += &generate_f64_xml("temperature", temperature);
    }
    if let Some(humidity) = pointcloud.humidity {
        xml += &generate_f64_xml("relativeHumidity", humidity);
    }
    if let Some(pressure) = pointcloud.atmospheric_pressure {
        xml += &generate_f64_xml("atmosphericPressure", pressure);
    }

    xml += &format!(
        "<points type=\"CompressedVector\" fileOffset=\"{}\" recordCount=\"{}\">\n",
        pointcloud.file_offset, pointcloud.records
    );
    xml += "<prototype type=\"Structure\">\n";
    for record in &pointcloud.prototype {
        xml += &record.xml_string();
    }
    xml += "</prototype>\n";
    xml += "</points>\n";
    xml += "</vectorChild>\n";
    Ok(xml)
}
