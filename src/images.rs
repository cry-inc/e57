use crate::error::Converter;
use crate::xml::{
    generate_float_xml, generate_int_xml, generate_string_xml, optional_date_time, optional_string,
    optional_transform, required_double, required_integer, required_string,
};
use crate::{Blob, DateTime, Error, Result, Transform};
use roxmltree::{Document, Node};

/// Descriptor with metadata for a single image.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Image {
    /// Globally unique identifier for the image object.
    pub guid: String,
    /// Preview/illustration image that does not define any camera projection model.
    pub visual_reference: Option<VisualReference>,
    /// Image that includes a projection model.
    pub representation: Option<Representation>,
    /// Transforms the local coordinate system of the image to the file-level coordinate system.
    pub transform: Option<Transform>,
    /// GUID of the pointcloud that was captured with this image.
    pub pointcloud_guid: Option<String>,
    /// User-defined name for the image.
    pub name: Option<String>,
    /// User-defined description for the image.
    pub description: Option<String>,
    /// Date and time when this image was captured.
    pub acquisition: Option<DateTime>,
    /// The name of the manufacturer for the sensor used to capture the image.
    pub sensor_vendor: Option<String>,
    /// The model name or number for the sensor used to capture the image.
    pub sensor_model: Option<String>,
    /// The serial number of the sensor used to capture the image.
    pub sensor_serial: Option<String>,
}

impl Image {
    fn from_node(node: &Node) -> Result<Self> {
        let guid = required_string(node, "guid")?;
        let pointcloud_guid = optional_string(node, "associatedData3DGuid")?;
        let transform = optional_transform(node, "pose")?;
        let name = optional_string(node, "name")?;
        let description = optional_string(node, "description")?;
        let sensor_model = optional_string(node, "sensorModel")?;
        let sensor_vendor = optional_string(node, "sensorVendor")?;
        let sensor_serial = optional_string(node, "sensorSerialNumber")?;
        let acquisition = optional_date_time(node, "acquisitionDateTime")?;
        let representation = Representation::from_image_node(node)?;

        let visual_reference_node = node
            .children()
            .find(|n| n.has_tag_name("visualReferenceRepresentation"));
        let visual_reference = if let Some(node) = visual_reference_node {
            Some(VisualReference::from_node(&node)?)
        } else {
            None
        };

        Ok(Self {
            guid,
            pointcloud_guid,
            transform,
            name,
            description,
            acquisition,
            sensor_vendor,
            sensor_model,
            sensor_serial,
            representation,
            visual_reference,
        })
    }

    pub(crate) fn vec_from_document(document: &Document) -> Result<Vec<Self>> {
        let images2d_node = document
            .descendants()
            .find(|n| n.has_tag_name("images2D"))
            .invalid_err("Cannot find 'images2D' tag in XML document")?;

        let mut images = Vec::new();
        for n in images2d_node.children() {
            if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
                let image = Self::from_node(&n)?;
                images.push(image);
            }
        }
        Ok(images)
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::new();
        xml += "<vectorChild type=\"Structure\">\n";
        xml += &generate_string_xml("guid", &self.guid);
        if let Some(vis_ref) = &self.visual_reference {
            xml += &vis_ref.xml_string();
        }
        if let Some(rep) = &self.representation {
            xml += &rep.xml_string();
        }
        if let Some(trans) = &self.transform {
            xml += &trans.xml_string("pose");
        }
        if let Some(pc_guid) = &self.pointcloud_guid {
            xml += &generate_string_xml("associatedData3DGuid", &pc_guid);
        }
        if let Some(name) = &self.name {
            xml += &generate_string_xml("name", &name);
        }
        if let Some(desc) = &self.description {
            xml += &generate_string_xml("description", &desc);
        }
        if let Some(acquisition) = &self.acquisition {
            xml += &acquisition.xml_string("acquisitionDateTime");
        }
        if let Some(vendor) = &self.sensor_vendor {
            xml += &generate_string_xml("sensorVendor", &vendor);
        }
        if let Some(model) = &self.sensor_model {
            xml += &generate_string_xml("sensorModel", &model);
        }
        if let Some(serial) = &self.sensor_serial {
            xml += &generate_string_xml("sensorSerialNumber", &serial);
        }
        xml += "</vectorChild>\n";
        xml
    }
}

/// Contains one of the tree possible types for projectable images.
#[derive(Debug, Clone)]
pub enum Representation {
    /// Image with a pinhole projection model.
    Pinhole(PinholeRepresentation),
    /// Image with a spherical projection model.
    Spherical(SphericalRepresentation),
    /// Image with a cylindrical projection model.
    Cylindrical(CylindricalRepresentation),
}

impl Representation {
    pub(crate) fn from_image_node(image_node: &Node) -> Result<Option<Self>> {
        let pinhole = image_node
            .children()
            .find(|n| n.has_tag_name("pinholeRepresentation"));
        if let Some(node) = &pinhole {
            return Ok(Some(Self::Pinhole(PinholeRepresentation::from_node(node)?)));
        }

        let spherical = image_node
            .children()
            .find(|n| n.has_tag_name("sphericalRepresentation"));
        if let Some(node) = &spherical {
            return Ok(Some(Self::Spherical(SphericalRepresentation::from_node(
                node,
            )?)));
        }

        let cylindrical = image_node
            .children()
            .find(|n| n.has_tag_name("cylindricalRepresentation"));
        if let Some(node) = &cylindrical {
            return Ok(Some(Self::Cylindrical(
                CylindricalRepresentation::from_node(node)?,
            )));
        }

        Ok(None)
    }

    pub(crate) fn xml_string(&self) -> String {
        match self {
            Representation::Pinhole(p) => p.xml_string(),
            Representation::Spherical(s) => s.xml_string(),
            Representation::Cylindrical(c) => c.xml_string(),
        }
    }
}

/// File format of an image stored inside the E57 file as blob.
#[derive(Debug, Clone)]
pub enum ImageFormat {
    /// Portable Network Graphics (PNG) image format.
    Png,
    /// JPEG File Interchange Format (JFIF) image format.
    Jpeg,
}

/// Contains a blob with image data and the corresponding file type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ImageBlob {
    /// Descriptor for the binary blob of the image.
    pub data: Blob,
    /// Image format of the file referenced by the blob.
    pub format: ImageFormat,
}

impl ImageBlob {
    pub(crate) fn from_rep_node(rep_node: &Node) -> Result<Self> {
        if let Some(node) = &rep_node.children().find(|n| n.has_tag_name("jpegImage")) {
            Ok(Self {
                data: Blob::from_node(node)?,
                format: ImageFormat::Jpeg,
            })
        } else if let Some(node) = &rep_node.children().find(|n| n.has_tag_name("pngImage")) {
            Ok(Self {
                data: Blob::from_node(node)?,
                format: ImageFormat::Png,
            })
        } else {
            Error::invalid("Cannot find PNG or JPEG blob")
        }
    }

    pub(crate) fn xml_string(&self) -> String {
        match self.format {
            ImageFormat::Png => self.data.xml_string("pngImage"),
            ImageFormat::Jpeg => self.data.xml_string("jpegImage"),
        }
    }
}

/// Properties of an visual reference image.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VisualReferenceProperties {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

/// A visual reference image for preview and illustration purposes.
///
/// Such images cannot be mapped to points and are not projectable!
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VisualReference {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Properties of the visual reference image.
    pub properties: VisualReferenceProperties,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
}

impl VisualReference {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            blob: ImageBlob::from_rep_node(node)?,
            mask: Blob::from_parent_node("imageMask", node)?,
            properties: VisualReferenceProperties {
                width: required_integer(node, "imageWidth")?,
                height: required_integer(node, "imageHeight")?,
            },
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::new();
        xml += "<visualReferenceRepresentation type=\"Structure\">\n";
        xml += &self.blob.xml_string();
        if let Some(mask) = &self.mask {
            xml += &mask.xml_string("imageMask");
        }
        xml += &generate_int_xml("imageWidth", self.properties.width);
        xml += &generate_int_xml("imageHeight", self.properties.height);
        xml += "</visualReferenceRepresentation>\n";
        xml
    }
}

/// Properties of a pinhole image.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PinholeImageProperties {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// The cameras focal length in meters.
    pub focal_length: f64,
    /// The width of a pixel in meters.
    pub pixel_width: f64,
    /// The height of a pixel in meters.
    pub pixel_height: f64,
    /// The X coordinate of the principal point in pixels.
    pub principal_x: f64,
    /// The Y coordinate of the principal point in pixels.
    pub principal_y: f64,
}

/// Describes an image with a pinhole camera projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PinholeRepresentation {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Properties of the pinhole image.
    pub properties: PinholeImageProperties,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
}

impl PinholeRepresentation {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            blob: ImageBlob::from_rep_node(node)?,
            mask: Blob::from_parent_node("imageMask", node)?,
            properties: PinholeImageProperties {
                width: required_integer(node, "imageWidth")?,
                height: required_integer(node, "imageHeight")?,
                focal_length: required_double(node, "focalLength")?,
                pixel_width: required_double(node, "pixelWidth")?,
                pixel_height: required_double(node, "pixelHeight")?,
                principal_x: required_double(node, "principalPointX")?,
                principal_y: required_double(node, "principalPointY")?,
            },
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::new();
        xml += "<pinholeRepresentation type=\"Structure\">\n";
        xml += &self.blob.xml_string();
        if let Some(mask) = &self.mask {
            xml += &mask.xml_string("imageMask");
        }
        xml += &generate_int_xml("imageWidth", self.properties.width);
        xml += &generate_int_xml("imageHeight", self.properties.height);
        xml += &generate_float_xml("focalLength", self.properties.focal_length);
        xml += &generate_float_xml("pixelWidth", self.properties.pixel_width);
        xml += &generate_float_xml("pixelHeight", self.properties.pixel_height);
        xml += &generate_float_xml("principalPointX", self.properties.principal_x);
        xml += &generate_float_xml("principalPointY", self.properties.principal_y);
        xml += "</pinholeRepresentation>\n";
        xml
    }
}

/// Properties of a spherical image.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SphericalImageProperties {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// The width of a pixel in radians.
    pub pixel_width: f64,
    /// The height of a pixel in radians.
    pub pixel_height: f64,
}

/// Describes an image with a spherical projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SphericalRepresentation {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Properties of the spherical image.
    pub properties: SphericalImageProperties,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
}

impl SphericalRepresentation {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            blob: ImageBlob::from_rep_node(node)?,
            mask: Blob::from_parent_node("imageMask", node)?,
            properties: SphericalImageProperties {
                width: required_integer(node, "imageWidth")?,
                height: required_integer(node, "imageHeight")?,
                pixel_width: required_double(node, "pixelWidth")?,
                pixel_height: required_double(node, "pixelHeight")?,
            },
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::new();
        xml += "<sphericalRepresentation type=\"Structure\">\n";
        xml += &self.blob.xml_string();
        if let Some(mask) = &self.mask {
            xml += &mask.xml_string("imageMask");
        }
        xml += &generate_int_xml("imageWidth", self.properties.width);
        xml += &generate_int_xml("imageHeight", self.properties.height);
        xml += &generate_float_xml("pixelWidth", self.properties.pixel_width);
        xml += &generate_float_xml("pixelHeight", self.properties.pixel_height);
        xml += "</sphericalRepresentation>\n";
        xml
    }
}

/// Properties of a cylindrical image.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CylindricalImageProperties {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// The closest distance from the cylindrical image surface to the center of projection in meters.
    pub radius: f64,
    /// The Y coordinate of the principal point in pixels.
    pub principal_y: f64,
    /// The width of a pixel in radians.
    pub pixel_width: f64,
    /// The height of a pixel in radians.
    pub pixel_height: f64,
}

/// Describes an image with a cylindrical projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CylindricalRepresentation {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Properties of the yylindrical image.
    pub properties: CylindricalImageProperties,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
}

impl CylindricalRepresentation {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            blob: ImageBlob::from_rep_node(node)?,
            mask: Blob::from_parent_node("imageMask", node)?,
            properties: CylindricalImageProperties {
                width: required_integer(node, "imageWidth")?,
                height: required_integer(node, "imageHeight")?,
                radius: required_double(node, "radius")?,
                principal_y: required_double(node, "principalPointY")?,
                pixel_width: required_double(node, "pixelWidth")?,
                pixel_height: required_double(node, "pixelHeight")?,
            },
        })
    }

    pub(crate) fn xml_string(&self) -> String {
        let mut xml = String::new();
        xml += "<cylindricalRepresentation type=\"Structure\">\n";
        xml += &self.blob.xml_string();
        if let Some(mask) = &self.mask {
            xml += &mask.xml_string("imageMask");
        }
        xml += &generate_int_xml("imageWidth", self.properties.width);
        xml += &generate_int_xml("imageHeight", self.properties.height);
        xml += &generate_float_xml("readius", self.properties.radius);
        xml += &generate_float_xml("principalPointY", self.properties.principal_y);
        xml += &generate_float_xml("pixelWidth", self.properties.pixel_width);
        xml += &generate_float_xml("pixelHeight", self.properties.pixel_height);
        xml += "</cylindricalRepresentation>\n";
        xml
    }
}
