//! Support for `pointGroupingSchemes/groupingByLine`: the optional
//! per-scan-line index that a structured point cloud may carry alongside its
//! points. Each group names one row or column and says where that line's
//! points start in the point vector and how many there are, so a reader can
//! seek to a single scan line without decoding the whole cloud.

use crate::error::Converter;
use crate::xml;
use crate::Error;
use crate::Record;
use crate::RecordDataType;
use crate::RecordName;
use crate::Result;
use roxmltree::Node;

/// One scan line: which row or column it is, where its points begin and how
/// many of them there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointGroup {
    /// Index into the point vector at which this line's points start.
    pub start_point_index: i64,
    /// The row or column index this group describes.
    pub id_element_value: i64,
    /// How many points belong to this line.
    pub point_count: i64,
}

/// The declared limits of the three group fields. E57 requires every integer
/// node to state its bounds, and a writer that has to match another
/// implementation may need to state something other than the obvious maxima,
/// so these are given explicitly rather than derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointGroupLimits {
    pub start_point_index_max: i64,
    pub id_element_value_max: i64,
    pub point_count_max: i64,
}

impl PointGroupLimits {
    /// The limits implied by the groups themselves: the largest value each
    /// field actually takes.
    pub fn from_groups(groups: &[PointGroup]) -> Self {
        Self {
            start_point_index_max: groups
                .iter()
                .map(|g| g.start_point_index)
                .max()
                .unwrap_or(0),
            id_element_value_max: groups.iter().map(|g| g.id_element_value).max().unwrap_or(0),
            point_count_max: groups.iter().map(|g| g.point_count).max().unwrap_or(0),
        }
    }
}

/// A `groupingByLine` scheme: the field the lines are keyed on, the groups
/// themselves, and the limits to declare for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointGroups {
    /// Either `rowIndex` or `columnIndex`.
    pub id_element_name: RecordName,
    pub groups: Vec<PointGroup>,
    pub limits: PointGroupLimits,
}

impl PointGroups {
    /// A scheme keyed on `id_element_name`, with the limits derived from the
    /// groups.
    pub fn new(id_element_name: RecordName, groups: Vec<PointGroup>) -> Self {
        let limits = PointGroupLimits::from_groups(&groups);
        Self {
            id_element_name,
            groups,
            limits,
        }
    }

    pub(crate) fn id_element_tag(&self) -> Result<&'static str> {
        match self.id_element_name {
            RecordName::RowIndex => Ok("rowIndex"),
            RecordName::ColumnIndex => Ok("columnIndex"),
            _ => Error::invalid("Point groups must be keyed on RowIndex or ColumnIndex"),
        }
    }

    /// The prototype of the groups compressed vector, in the order the E57
    /// standard names the fields.
    pub(crate) fn prototype(&self) -> Vec<Record> {
        vec![
            Record {
                name: RecordName::Unknown {
                    namespace: String::new(),
                    name: "startPointIndex".to_string(),
                },
                data_type: RecordDataType::Integer {
                    min: 0,
                    max: self.limits.start_point_index_max,
                },
            },
            Record {
                name: RecordName::Unknown {
                    namespace: String::new(),
                    name: "idElementValue".to_string(),
                },
                data_type: RecordDataType::Integer {
                    min: 0,
                    max: self.limits.id_element_value_max,
                },
            },
            Record {
                name: RecordName::Unknown {
                    namespace: String::new(),
                    name: "pointCount".to_string(),
                },
                data_type: RecordDataType::Integer {
                    min: 0,
                    max: self.limits.point_count_max,
                },
            },
        ]
    }
}

/// Where the groups of an existing file live, as read from its XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointGroupsHeader {
    pub id_element_name: RecordName,
    pub limits: PointGroupLimits,
    pub file_offset: u64,
    pub records: u64,
}

impl PointGroupsHeader {
    pub(crate) fn from_node(node: &Node) -> Result<Option<Self>> {
        let Some(schemes) = node
            .children()
            .find(|n| n.has_tag_name("pointGroupingSchemes"))
        else {
            return Ok(None);
        };
        let Some(by_line) = schemes
            .children()
            .find(|n| n.has_tag_name("groupingByLine"))
        else {
            return Ok(None);
        };

        let id_element_name = by_line
            .children()
            .find(|n| n.has_tag_name("idElementName"))
            .and_then(|n| n.text().map(|t| t.trim().to_string()))
            .invalid_err("Cannot find 'idElementName' in 'groupingByLine'")?;
        let id_element_name = match id_element_name.as_str() {
            "rowIndex" => RecordName::RowIndex,
            "columnIndex" => RecordName::ColumnIndex,
            other => Error::invalid(format!("Unknown group id element name: {other}"))?,
        };

        let groups = by_line
            .children()
            .find(|n| n.has_tag_name("groups"))
            .invalid_err("Cannot find 'groups' in 'groupingByLine'")?;
        let file_offset = groups
            .attribute("fileOffset")
            .invalid_err("Cannot find 'fileOffset' attribute in 'groups' tag")?
            .parse::<u64>()
            .invalid_err("Cannot parse 'fileOffset' attribute value as u64")?;
        let records = groups
            .attribute("recordCount")
            .invalid_err("Cannot find 'recordCount' attribute in 'groups' tag")?
            .parse::<u64>()
            .invalid_err("Cannot parse 'recordCount' attribute value as u64")?;

        let prototype = groups
            .children()
            .find(|n| n.has_tag_name("prototype"))
            .invalid_err("Cannot find 'prototype' child in 'groups' tag")?;
        let max_of = |name: &str| -> Result<i64> {
            prototype
                .children()
                .find(|n| n.has_tag_name(name))
                .and_then(|n| n.attribute("maximum").map(|a| a.to_string()))
                .invalid_err(format!(
                    "Cannot find 'maximum' of '{name}' in group prototype"
                ))?
                .parse::<i64>()
                .invalid_err(format!("Cannot parse 'maximum' of '{name}' as i64"))
        };

        Ok(Some(Self {
            id_element_name,
            limits: PointGroupLimits {
                start_point_index_max: max_of("startPointIndex")?,
                id_element_value_max: max_of("idElementValue")?,
                point_count_max: max_of("pointCount")?,
            },
            file_offset,
            records,
        }))
    }

    pub(crate) fn xml_string(&self) -> Result<String> {
        let id_element_name = match self.id_element_name {
            RecordName::RowIndex => "rowIndex",
            RecordName::ColumnIndex => "columnIndex",
            _ => Error::invalid("Point groups must be keyed on RowIndex or ColumnIndex")?,
        };
        let mut xml = String::new();
        xml += "<pointGroupingSchemes type=\"Structure\">\n";
        xml += "<groupingByLine type=\"Structure\">\n";
        xml += &xml::gen_string("idElementName", &id_element_name);
        xml += &format!(
            "<groups type=\"CompressedVector\" fileOffset=\"{}\" recordCount=\"{}\">\n",
            self.file_offset, self.records
        );
        xml += "<prototype type=\"Structure\">\n";
        xml += &format!(
            "<startPointIndex type=\"Integer\" minimum=\"0\" maximum=\"{}\"/>\n",
            self.limits.start_point_index_max
        );
        xml += &format!(
            "<idElementValue type=\"Integer\" minimum=\"0\" maximum=\"{}\"/>\n",
            self.limits.id_element_value_max
        );
        xml += &format!(
            "<pointCount type=\"Integer\" minimum=\"0\" maximum=\"{}\"/>\n",
            self.limits.point_count_max
        );
        xml += "</prototype>\n";
        xml += "<codecs type=\"Vector\" allowHeterogeneousChildren=\"1\">\n";
        xml += "</codecs>\n";
        xml += "</groups>\n";
        xml += "</groupingByLine>\n";
        xml += "</pointGroupingSchemes>\n";
        Ok(xml)
    }

    /// The prototype the group records were written with, so they can be
    /// decoded by the same machinery as points.
    pub(crate) fn prototype(&self) -> Vec<Record> {
        PointGroups {
            id_element_name: self.id_element_name.clone(),
            groups: Vec::new(),
            limits: self.limits,
        }
        .prototype()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn four_groups() -> Vec<PointGroup> {
        (0..4)
            .map(|i| PointGroup {
                start_point_index: i * 2,
                id_element_value: i,
                point_count: 2,
            })
            .collect()
    }

    #[test]
    fn limits_are_derived_from_the_groups() {
        let groups = PointGroups::new(RecordName::ColumnIndex, four_groups());
        assert_eq!(
            groups.limits,
            PointGroupLimits {
                start_point_index_max: 6,
                id_element_value_max: 3,
                point_count_max: 2,
            }
        );
    }

    #[test]
    fn limits_of_no_groups_are_zero() {
        let limits = PointGroupLimits::from_groups(&[]);
        assert_eq!(limits.start_point_index_max, 0);
        assert_eq!(limits.id_element_value_max, 0);
        assert_eq!(limits.point_count_max, 0);
    }

    #[test]
    fn the_prototype_declares_the_limits() {
        let groups = PointGroups::new(RecordName::RowIndex, four_groups());
        let prototype = groups.prototype();
        assert_eq!(prototype.len(), 3);
        assert!(matches!(
            prototype[2].data_type,
            RecordDataType::Integer { min: 0, max: 2 }
        ));
    }

    #[test]
    fn only_row_and_column_can_key_a_grouping() {
        assert_eq!(
            PointGroups::new(RecordName::RowIndex, Vec::new())
                .id_element_tag()
                .unwrap(),
            "rowIndex"
        );
        assert_eq!(
            PointGroups::new(RecordName::ColumnIndex, Vec::new())
                .id_element_tag()
                .unwrap(),
            "columnIndex"
        );
        assert!(PointGroups::new(RecordName::Intensity, Vec::new())
            .id_element_tag()
            .is_err());
    }

    #[test]
    fn the_generated_xml_round_trips_through_the_parser() {
        let header = PointGroupsHeader {
            id_element_name: RecordName::ColumnIndex,
            limits: PointGroupLimits {
                start_point_index_max: 6,
                id_element_value_max: 3,
                point_count_max: 2,
            },
            file_offset: 1234,
            records: 4,
        };
        let xml = format!("<pointcloud>{}</pointcloud>", header.xml_string().unwrap());
        let document = roxmltree::Document::parse(&xml).unwrap();
        let parsed = PointGroupsHeader::from_node(&document.root_element())
            .unwrap()
            .unwrap();
        assert_eq!(parsed, header);
    }
}
