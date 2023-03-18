use crate::blob::blob_from_node;
use crate::error::Converter;
use crate::xml::{
    optional_date_time, optional_string, optional_transform, required_double, required_integer,
    required_string,
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

/// A visual reference image for preview and illustration purposes.
///
/// Such images cannot be mapped to points and are not projectable!
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VisualReference {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

/// Describes an image with a pinhole camera projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PinholeRepresentation {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
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

/// Describes an image with a spherical projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SphericalRepresentation {
    /// Reference to the binary image data.
    pub blob: ImageBlob,
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
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
    /// Reference to a PNG image with a mask for non-rectangular images.
    ///
    /// The mask is used to indicate which pixels in the image are valid.
    /// The mask dimension are the same as the image itself.
    /// It has non-zero-valued pixels at locations where the image is valid
    /// and zero-valued pixels at locations where it is invalid.
    pub mask: Option<Blob>,
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

pub fn images_from_document(document: &Document) -> Result<Vec<Image>> {
    let images2d_node = document
        .descendants()
        .find(|n| n.has_tag_name("images2D"))
        .invalid_err("Cannot find 'images2D' tag in XML document")?;

    let mut images = Vec::new();
    for n in images2d_node.children() {
        if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
            let image = image_from_node(&n)?;
            images.push(image);
        }
    }
    Ok(images)
}

fn image_from_node(node: &Node) -> Result<Image> {
    let guid = required_string(node, "guid")?;
    let pointcloud_guid = optional_string(node, "associatedData3DGuid")?;
    let transform = optional_transform(node, "pose")?;
    let name = optional_string(node, "name")?;
    let description = optional_string(node, "description")?;
    let sensor_model = optional_string(node, "sensorModel")?;
    let sensor_vendor = optional_string(node, "sensorVendor")?;
    let sensor_serial = optional_string(node, "sensorSerialNumber")?;
    let acquisition = optional_date_time(node, "acquisitionDateTime")?;

    let visual_reference_node = node
        .children()
        .find(|n| n.has_tag_name("visualReferenceRepresentation"));
    let visual_reference = if let Some(node) = visual_reference_node {
        Some(visual_reference_from_node(&node)?)
    } else {
        None
    };

    let representation = extract_representation(node)?;

    Ok(Image {
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

fn extract_representation(parent_node: &Node) -> Result<Option<Representation>> {
    let pinhole = parent_node
        .children()
        .find(|n| n.has_tag_name("pinholeRepresentation"));
    if let Some(node) = &pinhole {
        return Ok(Some(Representation::Pinhole(pinhole_rep_from_node(node)?)));
    }

    let spherical = parent_node
        .children()
        .find(|n| n.has_tag_name("sphericalRepresentation"));
    if let Some(node) = &spherical {
        return Ok(Some(Representation::Spherical(spherical_rep_from_node(
            node,
        )?)));
    }

    let cylindrical = parent_node
        .children()
        .find(|n| n.has_tag_name("cylindricalRepresentation"));
    if let Some(node) = &cylindrical {
        return Ok(Some(Representation::Cylindrical(
            cylindrical_rep_from_node(node)?,
        )));
    }

    Ok(None)
}

fn visual_reference_from_node(node: &Node) -> Result<VisualReference> {
    Ok(VisualReference {
        blob: extract_image_blob(node)?,
        mask: extract_mask_blob(node)?,
        width: required_integer(node, "imageWidth")?,
        height: required_integer(node, "imageHeight")?,
    })
}

fn pinhole_rep_from_node(node: &Node) -> Result<PinholeRepresentation> {
    Ok(PinholeRepresentation {
        blob: extract_image_blob(node)?,
        mask: extract_mask_blob(node)?,
        width: required_integer(node, "imageWidth")?,
        height: required_integer(node, "imageHeight")?,
        focal_length: required_double(node, "focalLength")?,
        pixel_width: required_double(node, "pixelWidth")?,
        pixel_height: required_double(node, "pixelHeight")?,
        principal_x: required_double(node, "principalPointX")?,
        principal_y: required_double(node, "principalPointY")?,
    })
}

fn spherical_rep_from_node(node: &Node) -> Result<SphericalRepresentation> {
    let blob = extract_image_blob(node)?;
    let mask = extract_mask_blob(node)?;
    let width = required_integer(node, "imageWidth")?;
    let height = required_integer(node, "imageHeight")?;
    let pixel_width = required_double(node, "pixelWidth")?;
    let pixel_height = required_double(node, "pixelHeight")?;
    Ok(SphericalRepresentation {
        blob,
        mask,
        width,
        height,
        pixel_width,
        pixel_height,
    })
}

fn cylindrical_rep_from_node(node: &Node) -> Result<CylindricalRepresentation> {
    let blob = extract_image_blob(node)?;
    let mask = extract_mask_blob(node)?;
    let width = required_integer(node, "imageWidth")?;
    let height = required_integer(node, "imageHeight")?;
    let radius = required_double(node, "radius")?;
    let principal_y = required_double(node, "principalPointY")?;
    let pixel_width = required_double(node, "pixelWidth")?;
    let pixel_height = required_double(node, "pixelHeight")?;
    Ok(CylindricalRepresentation {
        blob,
        mask,
        width,
        height,
        radius,
        principal_y,
        pixel_width,
        pixel_height,
    })
}

fn extract_image_blob(parent_node: &Node) -> Result<ImageBlob> {
    if let Some(node) = &parent_node.children().find(|n| n.has_tag_name("jpegImage")) {
        Ok(ImageBlob {
            data: blob_from_node(node)?,
            format: ImageFormat::Jpeg,
        })
    } else if let Some(node) = &parent_node.children().find(|n| n.has_tag_name("pngImage")) {
        Ok(ImageBlob {
            data: blob_from_node(node)?,
            format: ImageFormat::Png,
        })
    } else {
        Error::invalid("Cannot find PNG or JPEG blob")
    }
}

fn extract_mask_blob(parent_node: &Node) -> Result<Option<Blob>> {
    if let Some(node) = &parent_node.children().find(|n| n.has_tag_name("imageMask")) {
        Ok(Some(blob_from_node(node)?))
    } else {
        Ok(None)
    }
}
