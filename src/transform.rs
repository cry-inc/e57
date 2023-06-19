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

impl Quaternion {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        let w = required_double(node, "w")?;
        let x = required_double(node, "x")?;
        let y = required_double(node, "y")?;
        let z = required_double(node, "z")?;
        Ok(Self { w, x, y, z })
    }
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

impl Translation {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        let x = required_double(node, "x")?;
        let y = required_double(node, "y")?;
        let z = required_double(node, "z")?;
        Ok(Self { x, y, z })
    }
}

/// Describes a transformation of a point cloud with a rotation and translation component.
#[derive(Clone, Debug)]
pub struct Transform {
    /// A unit quaternion representing the rotation of the transform.
    pub rotation: Quaternion,
    /// The translation of the transform.
    pub translation: Translation,
}

impl Transform {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
        let translation = node
            .children()
            .find(|n| n.has_tag_name("translation"))
            .invalid_err("Cannot find translation tag of transform")?;
        let quaternion = node
            .children()
            .find(|n| n.has_tag_name("rotation"))
            .invalid_err("Cannot find quaternion tag of transform")?;
        Ok(Self {
            rotation: Quaternion::from_node(&quaternion)?,
            translation: Translation::from_node(&translation)?,
        })
    }

    pub(crate) fn xml_string(&self, tag_name: &str) -> String {
        let w = generate_f64_xml("w", self.rotation.w);
        let x = generate_f64_xml("x", self.rotation.x);
        let y = generate_f64_xml("y", self.rotation.y);
        let z = generate_f64_xml("z", self.rotation.z);
        let quat = format!("<rotation type=\"Structure\">{w}{x}{y}{z}</rotation>\n");

        let x = generate_f64_xml("x", self.translation.x);
        let y = generate_f64_xml("y", self.translation.y);
        let z = generate_f64_xml("z", self.translation.z);
        let trans = format!("<translation type=\"Structure\">{x}{y}{z}</translation>\n");

        format!("<{tag_name} type=\"Structure\">{quat}{trans}</{tag_name}>\n")
    }
}
