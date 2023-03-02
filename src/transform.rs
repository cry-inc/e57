use crate::{error::Converter, Result};
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

fn float_value_from_child(node: &Node, child_name: &str) -> Result<f64> {
    node.children()
        .find(|n| n.has_tag_name(child_name))
        .invalid_err(format!("Cannot find tag with name '{child_name}'"))?
        .text()
        .unwrap_or("0")
        .parse::<f64>()
        .invalid_err(format!("Cannot parse '{child_name}' value"))
}

pub fn quaternion_from_node(node: &Node) -> Result<Quaternion> {
    let w = float_value_from_child(node, "w")?;
    let x = float_value_from_child(node, "x")?;
    let y = float_value_from_child(node, "y")?;
    let z = float_value_from_child(node, "z")?;
    Ok(Quaternion { w, x, y, z })
}

pub fn translation_from_node(node: &Node) -> Result<Translation> {
    let x = float_value_from_child(node, "x")?;
    let y = float_value_from_child(node, "y")?;
    let z = float_value_from_child(node, "z")?;
    Ok(Translation { x, y, z })
}
