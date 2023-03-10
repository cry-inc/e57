use crate::{date_time::date_time_from_node, error::Converter, DateTime, Error, Result};
use roxmltree::Node;
use std::str::FromStr;

pub fn optional_string(parent_node: &Node, tag_name: &str) -> Result<Option<String>> {
    if let Some(tag) = parent_node.children().find(|n| n.has_tag_name(tag_name)) {
        let expected_type = "String";
        if let Some(found_type) = tag.attribute("type") {
            if found_type != expected_type {
                Error::invalid(format!(
                    "Found XML tag '{tag_name}' with type '{found_type}' instead of '{expected_type}'"
                ))?
            }
        } else {
            Error::invalid(format!("XML tag '{tag_name}' has no 'type' attribute"))?
        }
        let text = tag.text().unwrap_or("");
        Ok(Some(text.to_string()))
    } else {
        Ok(None)
    }
}

pub fn required_string(parent_node: &Node, tag_name: &str) -> Result<String> {
    let str = optional_string(parent_node, tag_name)?;
    str.invalid_err(format!("XML tag '{tag_name}' was not found"))
}

fn optional_number<T: FromStr + Sync + Send>(
    parent_node: &Node,
    tag_name: &str,
    expected_type: &str,
) -> Result<Option<T>> {
    if let Some(tag) = parent_node.children().find(|n| n.has_tag_name(tag_name)) {
        if let Some(found_type) = tag.attribute("type") {
            if found_type != expected_type {
                Error::invalid(format!(
                    "Found XML tag '{tag_name}' with type '{found_type}' instead of '{expected_type}'"
                ))?
            }
        } else {
            Error::invalid(format!("XML tag '{tag_name}' has no 'type' attribute"))?
        }
        let text = tag.text().unwrap_or("0");
        if let Ok(parsed) = text.parse::<T>() {
            Ok(Some(parsed))
        } else {
            Error::invalid(format!(
                "Cannot parse value '{text}' of XML tag '{tag_name}' as '{expected_type}'"
            ))?
        }
    } else {
        Ok(None)
    }
}

pub fn optional_double(parent_node: &Node, tag_name: &str) -> Result<Option<f64>> {
    optional_number(parent_node, tag_name, "Float")
}

pub fn required_double(parent_node: &Node, tag_name: &str) -> Result<f64> {
    let double = optional_number(parent_node, tag_name, "Float")?;
    double.invalid_err(format!("XML tag '{tag_name}' was not found"))
}

pub fn optional_integer(parent_node: &Node, tag_name: &str) -> Result<Option<i64>> {
    optional_number(parent_node, tag_name, "Integer")
}

pub fn required_integer(parent_node: &Node, tag_name: &str) -> Result<i64> {
    let integer = optional_number(parent_node, tag_name, "Integer")?;
    integer.invalid_err(format!("XML tag '{tag_name}' was not found"))
}

pub fn optional_date_time(parent_node: &Node, tag_name: &str) -> Result<Option<DateTime>> {
    if let Some(tag) = parent_node.children().find(|n| n.has_tag_name(tag_name)) {
        let expected_type = "Structure";
        if let Some(found_type) = tag.attribute("type") {
            if found_type != expected_type {
                Error::invalid(format!(
                    "Found XML tag '{tag_name}' with type '{found_type}' instead of '{expected_type}'"
                ))?
            }
        } else {
            Error::invalid(format!("XML tag '{tag_name}' has no 'type' attribute"))?
        }
        date_time_from_node(&tag)
    } else {
        Ok(None)
    }
}
