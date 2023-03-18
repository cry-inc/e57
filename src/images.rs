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
    pub guid: String,
    pub visual_reference: Option<VisualReference>,
    pub representation: Option<Representation>,
    pub transform: Option<Transform>,
    pub pointcloud_guid: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub acquisition: Option<DateTime>,
    pub sensor_vendor: Option<String>,
    pub sensor_model: Option<String>,
    pub sensor_serial: Option<String>,
}

/// Contains one of the tree possible types for projectable images.
#[derive(Debug, Clone)]
pub enum Representation {
    Pinhole(PinholeRepresentation),
    Spherical(SphericalRepresentation),
    Cylindrical(CylindricalRepresentation),
}

/// File format of an image stored inside the E57 file as blob.
#[derive(Debug, Clone)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

/// Contains a blob with image data and the corresponding file type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ImageBlob {
    pub data: Blob,
    pub format: ImageFormat,
}

/// A visual reference image for preview and illustration purposes.
///
/// Such images cannot be mapped to points and are not projectable!
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VisualReference {
    pub blob: ImageBlob,
    pub mask: Option<Blob>,
    pub width: u32,
    pub height: u32,
}

/// Describes an image with a pinhole camera projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PinholeRepresentation {
    pub blob: ImageBlob,
    pub mask: Option<Blob>,
    pub width: u32,
    pub height: u32,
    pub focal_length: f64,
    pub pixel_width: f64,
    pub pixel_height: f64,
    pub principal_x: f64,
    pub principal_y: f64,
}

/// Describes an image with a spherical projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SphericalRepresentation {
    pub blob: ImageBlob,
    pub mask: Option<Blob>,
    pub width: u32,
    pub height: u32,
    pub pixel_width: f64,
    pub pixel_height: f64,
}

/// Describes an image with a cylindrical projection model.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CylindricalRepresentation {
    pub blob: ImageBlob,
    pub mask: Option<Blob>,
    pub width: u32,
    pub height: u32,
    pub radius: f64,
    pub principal_y: f64,
    pub pixel_width: f64,
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
