use crate::error::Converter;
use crate::xml::{generate_f64_xml, required_double};
use crate::Result;
use roxmltree::Node;

/// Describes the rotation of a point cloud.
#[derive(Clone, Debug)]
pub struct Quaternion {
    /// The scalar part of the quaternion. Shall be nonnegative.
    pub w: f64,
    /// The i coefficient of the quaternion.
    pub x: f64,
    /// The j coefficient of the quaternion.
    pub y: f64,
    /// The k coefficient of the quaternion.
    pub z: f64,
}

/// Describes the translation of a point cloud.
#[derive(Clone, Debug)]
pub struct Translation {
    /// The X coordinate of the translation in meters.
    pub x: f64,
    /// The Y coordinate of the translation in meters.
    pub y: f64,
    /// The Z coordinate of the translation in meters.
    pub z: f64,
}

/// Describes a transformation of a point cloud with a rotation and translation component.
#[derive(Clone, Debug)]
pub struct Transform {
    /// A unit quaternion representing the rotation of the transform.
    pub rotation: Quaternion,
    /// The translation of the transform.
    pub translation: Translation,
}

pub fn transform_from_node(node: &Node) -> Result<Transform> {
    let translation = node
        .children()
        .find(|n| n.has_tag_name("translation"))
        .invalid_err("Cannot find translation tag of transform")?;
    let quaternion = node
        .children()
        .find(|n| n.has_tag_name("rotation"))
        .invalid_err("Cannot find quaternion tag of transform")?;
    Ok(Transform {
        rotation: quaternion_from_node(&quaternion)?,
        translation: translation_from_node(&translation)?,
    })
}

pub fn quaternion_from_node(node: &Node) -> Result<Quaternion> {
    let w = required_double(node, "w")?;
    let x = required_double(node, "x")?;
    let y = required_double(node, "y")?;
    let z = required_double(node, "z")?;
    Ok(Quaternion { w, x, y, z })
}

pub fn translation_from_node(node: &Node) -> Result<Translation> {
    let x = required_double(node, "x")?;
    let y = required_double(node, "y")?;
    let z = required_double(node, "z")?;
    Ok(Translation { x, y, z })
}

pub fn serialize_transform(transform: &Transform, tag_name: &str) -> String {
    let w = generate_f64_xml("w", transform.rotation.w);
    let x = generate_f64_xml("x", transform.rotation.x);
    let y = generate_f64_xml("y", transform.rotation.y);
    let z = generate_f64_xml("z", transform.rotation.z);
    let quat = format!("<rotation>{w}{x}{y}{z}</rotation>\n");

    let x = generate_f64_xml("x", transform.translation.x);
    let y = generate_f64_xml("y", transform.translation.y);
    let z = generate_f64_xml("z", transform.translation.z);
    let trans = format!("<translation>{x}{y}{z}</translation>\n");

    format!("<{tag_name}>{quat}{trans}</{tag_name}>\n")
}
