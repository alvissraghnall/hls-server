use std::{
    collections::HashMap,
    fmt::{self},
    str::FromStr,
};

use chrono::{DateTime, FixedOffset};

use crate::{
    attribute_list::{AttributeList, AttributeValue, parse_attribute_list},
    error::{ParseError, Span},
    segment::parse_datetime,
    uri::Uri,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SharedTag {
    Version(u8),
    Variable(PlayListVariableDefinition),
    IndependentSegments,
    Start { precise: bool, time_offset: f64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayListVariableDefinition {
    NameValue { name: String, value: String },
    Import { name: String },
    QueryParam { name: String, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaMetadata {
    Daterange(DateRange),
    Skip(Skip),
    PreloadHint(PreloadHint),
    RenditionReport(RenditionReport),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Skip {
    skipped_segments: u64,
    pub(crate) recently_removed_dateranges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenditionReport {
    uri: Uri,
    last_msn: u64,
    last_part: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreloadHint {
    hint_type: PreloadHintType,
    uri: Uri,
    byterange_start: u64,
    byterange_length: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreloadHintType {
    Map,
    Part,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DateRange {
    id: String,
    class: Option<String>,
    start_date: Option<DateTime<FixedOffset>>,
    cue: Vec<Cue>,
    end_date: Option<DateTime<FixedOffset>>,
    duration: Option<f64>,
    planned_duration: Option<f64>,
    end_on_next: Option<bool>,

    extensions: HashMap<String, AttributeValue>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Cue {
    Pre,
    Post,
    Once,
}

impl Default for SharedTag {
    fn default() -> Self {
        Self::Version(1)
    }
}

impl PlayListVariableDefinition {
    pub fn new(name: &str, value: &str) -> Self {
        Self::NameValue {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        match self {
            Self::NameValue { name, .. } => name.as_str(),
            Self::Import { name, .. } => name.as_str(),
            Self::QueryParam { name, .. } => name.as_str(),
        }
    }
}

impl TryFrom<AttributeList> for PlayListVariableDefinition {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let has_import = map.contains_key("IMPORT");
        let has_query = map.contains_key("QUERY");
        let has_namevalue = map.contains_key("NAME") || map.contains_key("VALUE");

        let count = has_import as u8 + has_query as u8 + has_namevalue as u8;

        match count {
            0 => {
                return Err(ParseError::MissingAttribute {
                    attribute: "IMPORT, QUERY, or NAME/VALUE".into(),
                });
            }
            2.. => {
                return Err(ParseError::TooManyAttributes {
                    expected: 1,
                    found: count as usize,
                });
            }
            _ => {}
        }

        if has_import {
            let raw = map
                .remove("IMPORT")
                .ok_or(ParseError::UnknownAttribute("IMPORT".to_string()))?;

            return Ok(PlayListVariableDefinition::Import {
                name: raw
                    .as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)?
                    .to_string(),
            });
        }

        if has_query {
            let raw = map
                .remove("QUERY")
                .ok_or(ParseError::UnknownAttribute("QUERY".to_string()))?;

            return Ok(PlayListVariableDefinition::QueryParam {
                name: raw
                    .as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)?
                    .to_string(),
                value: String::new(),
            });
        }

        if has_namevalue {
            let raw_name = map
                .remove("NAME")
                .ok_or(ParseError::UnknownAttribute("NAME".to_string()))?;
            let raw_value = map
                .remove("VALUE")
                .ok_or(ParseError::UnknownAttribute("VALUE".to_string()))?;

            return Ok(PlayListVariableDefinition::NameValue {
                name: raw_name
                    .as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)?
                    .to_string(),
                value: raw_value
                    .as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)?
                    .to_string(),
            });
        }

        Err(ParseError::MissingAttribute {
            attribute: "IMPORT, QUERY, or NAME/VALUE".into(),
        })
    }
}

impl MediaMetadata {
    pub fn parse_line(line: &str, line_number: usize) -> Result<Self, ParseError> {
        match line {
            l if l.starts_with("#EXT-X-DATERANGE:") => {
                let content = &l["#EXT-X-DATERANGE:".len()..];
                let attributes = parse_attribute_list(content)?;
                Ok(MediaMetadata::Daterange(DateRange::try_from(attributes)?))
            }
            l if l.starts_with("#EXT-X-SKIP:") => {
                let content = &l["#EXT-X-SKIP:".len()..];
                let attributes = parse_attribute_list(content)?;
                Ok(MediaMetadata::Skip(Skip::try_from(attributes)?))
            }
            l if l.starts_with("#EXT-X-PRELOAD-HINT:") => {
                let content = &l["#EXT-X-PRELOAD-HINT:".len()..];
                let attributes = parse_attribute_list(content)?;
                Ok(MediaMetadata::PreloadHint(PreloadHint::try_from(
                    attributes,
                )?))
            }
            l if l.starts_with("#EXT-X-RENDITION-REPORT:") => {
                let content = &l["#EXT-X-RENDITION-REPORT:".len()..];
                let attributes = parse_attribute_list(content)?;
                Ok(MediaMetadata::RenditionReport(RenditionReport::try_from(
                    attributes,
                )?))
            }
            _ => Err(ParseError::UnknownTag {
                tag: line.to_string(),
                span: Span {
                    line: line_number,
                    column: 0,
                },
            }),
        }
    }
}

impl TryFrom<AttributeList> for DateRange {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let id = map
            .remove("ID")
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "ID".into(),
                value: "NONE".into(),
                expected: "a quoted string".into(),
            })?
            .as_quoted_string()
            .ok_or(ParseError::ExpectedQuotedString)?
            .to_string();

        let class = map
            .remove("CLASS")
            .map(|val| {
                val.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|s| s.to_string())
            })
            .transpose()?;

        let start_date = map
            .remove("START-DATE")
            .map(|val| {
                val.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|s| {
                        parse_datetime(s).map_err(|_| ParseError::InvalidAttributeValue {
                            attribute: "START-DATE".into(),
                            value: s.into(),
                            expected: "a valid datetime quoted string".into(),
                        })
                    })
            })
            .transpose()?;

        let cue = map
            .remove("CUE")
            .map(|val| {
                val.as_enumerated_string_list()
                    .ok_or_else(|| ParseError::InvalidAttributeValue {
                        attribute: "CUE".into(),
                        value: "NONE".into(),
                        expected: "a list of enumerated strings".into(),
                    })?
                    .iter()
                    .map(|x| Cue::from_str(x))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or(vec![]);

        let end_date = map
            .remove("END-DATE")
            .map(|val| {
                val.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|s| {
                        parse_datetime(s).map_err(|_| ParseError::InvalidAttributeValue {
                            attribute: "END-DATE".into(),
                            value: s.into(),
                            expected: "a valid datetime quoted string for as described in RFC 8216"
                                .into(),
                        })
                    })
            })
            .transpose()?;

        let duration = map
            .remove("DURATION")
            .map(|val| {
                val.as_decimal_floating_point()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "DURATION".into(),
                        value: "NONE".into(),
                        expected: "a decimal floating point number".into(),
                    })
            })
            .transpose()?;

        let planned_duration = map
            .remove("PLANNED-DURATION")
            .map(|val| {
                val.as_decimal_floating_point()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "PLANNED-DURATION".into(),
                        value: "NONE".into(),
                        expected: "a positive decimal floating point number".into(),
                    })
                    .and_then(|x| {
                        if x < 0.0 {
                            Err(ParseError::InvalidAttributeValue {
                                attribute: "PLANNED-DURATION".into(),
                                value: x.to_string().into(),
                                expected: "a positive decimal floating point number".into(),
                            })
                        } else {
                            Ok(x)
                        }
                    })
            })
            .transpose()?;

        let x_attr = map
            .iter()
            .filter_map(|(k, v)| {
                if k.starts_with("X-") {
                    Some((k.to_string(), v.clone()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let end_on_next = map
            .remove("END-ON-NEXT")
            .map(|val| {
                val.as_enumerated_string()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "END-ON-NEXT".into(),
                        value: "NONE".into(),
                        expected: "an enumerated string".into(),
                    })
                    .map(|s| Some(s == "YES"))
            })
            .transpose()?
            .unwrap_or(None);

        // scte-35 curr unhandled.

        Ok(Self {
            id,
            class,
            start_date,
            cue: cue,
            end_date,
            duration,
            planned_duration,
            end_on_next,
            extensions: x_attr,
        })

        // Err(ParseError::UnknownAttribute("ID".to_string()))
    }
}

impl FromStr for Cue {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PRE" => Ok(Cue::Pre),
            "POST" => Ok(Cue::Post),
            "ONCE" => Ok(Cue::Once),
            _ => Err(ParseError::InvalidAttributeValue {
                attribute: "CUE".into(),
                value: s.into(),
                expected: "an enumerated string with value PRE, POST, or ONCE",
            }),
        }
    }
}

impl TryInto<Cue> for AttributeValue {
    type Error = ParseError;

    fn try_into(self) -> Result<Cue, Self::Error> {
        let list = self
            .as_enumerated_string_list()
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "CUE".into(),
                value: self.to_string().into(),
                expected: "an enumerated string with value PRE, POST, or ONCE".into(),
            })?;

        if let Some(v) = list.get("PRE") {
            return Ok(Cue::Pre);
        }

        if let Some(v) = list.get("POST") {
            return Ok(Cue::Post);
        }

        if let Some(v) = list.get("ONCE") {
            return Ok(Cue::Once);
        }

        Err(ParseError::InvalidAttributeValue {
            attribute: "CUE".into(),
            value: self.to_string().into(),
            expected: "an enumerated string with value PRE, POST, or ONCE".into(),
        })
    }
}

impl TryFrom<AttributeList> for PreloadHint {
    type Error = ParseError;

    fn try_from(value: AttributeList) -> Result<Self, Self::Error> {
        let type_of = value
            .get("TYPE")
            .and_then(|v| v.as_enumerated_string())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "TYPE".into(),
                value: "NONE".into(),
                expected: "an enumerated string".into(),
            })
            .and_then(|s| match s {
                "MAP" => Ok(PreloadHintType::Map),
                "PART" => Ok(PreloadHintType::Part),
                _ => Err(ParseError::InvalidAttributeValue {
                    attribute: "TYPE".into(),
                    value: s.into(),
                    expected: "an enumerated string with value MAP or PART".into(),
                }),
            })?;

        let uri = value
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "URI".into(),
                value: "NONE".into(),
                expected: "a quoted string".into(),
            })
            .and_then(|s| {
                Uri::from_str(&s).map_err(|_| ParseError::InvalidAttributeValue {
                    attribute: "URI".into(),
                    value: s.into(),
                    expected: "a valid URI".into(),
                })
            })?;

        let byterange_start = value
            .get("BYTERANGE-START")
            .and_then(|v| v.as_decimal_integer())
            .unwrap_or(0);

        let byterange_length = value
            .get("BYTERANGE-LENGTH")
            .and_then(|v| v.as_decimal_integer());

        Ok(Self {
            hint_type: type_of,
            uri,
            byterange_start,
            byterange_length,
        })
    }
}

impl TryFrom<AttributeList> for RenditionReport {
    type Error = ParseError;

    fn try_from(value: AttributeList) -> Result<Self, Self::Error> {
        let uri = value
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "URI".into(),
                value: "NONE".into(),
                expected: "a quoted string".into(),
            })
            .and_then(|s| {
                Uri::from_str(&s).map_err(|_| ParseError::InvalidAttributeValue {
                    attribute: "URI".into(),
                    value: s.into(),
                    expected: "a valid URI".into(),
                })
            })?;

        let last_msn = value
            .get("LAST-MSN")
            .and_then(|v| v.as_decimal_integer())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "LAST-MSN".into(),
                value: "NONE".into(),
                expected: "a valid decimal integer".into(),
            })?;

        let last_part = value.get("LAST-PART").and_then(|v| v.as_decimal_integer());

        Ok(Self {
            uri,
            last_msn,
            last_part,
        })
    }
}

impl TryFrom<AttributeList> for Skip {
    type Error = ParseError;

    fn try_from(value: AttributeList) -> Result<Self, Self::Error> {
        let skipped_segments = value
            .get("SKIPPED-SEGMENTS")
            .and_then(|v| v.as_decimal_integer())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "SKIPPED-SEGMENTS".into(),
                value: "NONE".into(),
                expected: "a valid decimal integer".into(),
            })?;

        let recently_removed_dateranges = value
            .get("RECENTLY-REMOVED-DATERANGES")
            .and_then(|v| v.as_quoted_string())
            .map(|s| s.split('\t').map(|s| s.trim().to_string()).collect())
            .unwrap_or(vec![]);

        Ok(Self {
            skipped_segments,
            recently_removed_dateranges,
        })
    }
}

impl fmt::Display for PlayListVariableDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayListVariableDefinition::NameValue { name, value } => {
                write!(f, r#"NAME="{}",VALUE="{}""#, name, value)
            }
            PlayListVariableDefinition::Import { name } => {
                write!(f, r#"IMPORT="{}""#, name)
            }
            PlayListVariableDefinition::QueryParam { name, value: _ } => {
                write!(f, r#"QUERYPARAM="{}""#, name)
            }
        }
    }
}

impl MediaMetadata {
    pub(crate) fn as_skip(&self) -> Option<&Skip> {
        match self {
            MediaMetadata::Skip(skip) => Some(skip),
            _ => None,
        }
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "#EXT-X-SKIP:SKIPPED-SEGMENTS={}", self.skipped_segments)?;
        write!(f, ",RECENTLY-REMOVED-DATERANGES=\"")?;

        for (i, daterange_id) in self.recently_removed_dateranges.iter().enumerate() {
            if i > 0 {
                write!(f, "\t")?;
            }
            write!(f, "{daterange_id}")?;
        }

        write!(f, "\"")?;

        Ok(())
    }
}

impl fmt::Display for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cue::Pre => write!(f, "PRE")?,
            Cue::Post => write!(f, "POST")?,
            Cue::Once => write!(f, "ONCE")?,
        }

        Ok(())
    }
}

impl fmt::Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DateRange {
            id,
            class,
            start_date,
            cue,
            end_date,
            duration,
            planned_duration,
            end_on_next,
            extensions,
        } = self;

        write!(f, "#EXT-X-DATERANGE:ID={}", id)?;
        if let Some(class) = class {
            write!(f, ",CLASS=\"{class}\"")?;
        }
        if let Some(start) = start_date {
            write!(f, ",START-DATE=\"{start}\"")?;
        }

        write!(f, ",CUE=\"")?;

        for (i, trigger_id) in cue.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{trigger_id}")?;
        }

        write!(f, "\"")?;
        if let Some(end) = end_date {
            write!(f, ",END-DATE=\"{end}\"")?;
        }
        if let Some(duration) = duration {
            write!(f, ",DURATION={duration}")?;
        }
        if let Some(planned_duration) = planned_duration {
            write!(f, ",PLANNED-DURATION={planned_duration}")?;
        }
        if let Some(end_on_next) = end_on_next
            && *end_on_next
        {
            write!(f, ",END-ON-NEXT=YES")?;
        }
        for (key, value) in extensions.iter() {
            write!(f, ",{key}=\"{value}\"")?;
        }

        Ok(())
    }
}

impl fmt::Display for PreloadHintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreloadHintType::Map => write!(f, "MAP"),
            PreloadHintType::Part => write!(f, "PART"),
        }
    }
}

impl fmt::Display for PreloadHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#EXT-X-PRELOAD-HINT:TYPE={}", self.hint_type)?;
        write!(f, ",URI=\"{}\"", self.uri)?;
        write!(f, ",BYTERANGE-START={}", self.byterange_start)?;
        if let Some(byterange_len) = &self.byterange_length {
            write!(f, ",BYTERANGE-LENGTH={byterange_len}")?;
        }

        Ok(())
    }
}

impl fmt::Display for RenditionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#EXT-X-RENDITION-REPORT:")?;
        write!(f, "URI=\"{}\"", self.uri)?;
        write!(f, ",LAST-MSN={}", self.last_msn)?;
        if let Some(last_part) = &self.last_part {
            write!(f, ",LAST-PART={last_part}")?;
        }

        Ok(())
    }
}

impl fmt::Display for MediaMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaMetadata::Skip(skip) => write!(f, "{}", skip),
            MediaMetadata::Daterange(daterange) => write!(f, "{}", daterange),
            MediaMetadata::PreloadHint(preload_hint) => write!(f, "{}", preload_hint),
            MediaMetadata::RenditionReport(rendition_report) => write!(f, "{}", rendition_report),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_list::{AttributeList, AttributeValue};

    #[test]
    fn test_variable_def_from_attributes_name_value() {
        let mut attrs = AttributeList::new();
        attrs.insert(
            "NAME".to_string(),
            AttributeValue::QuotedString("VAR".to_string()),
        );
        attrs.insert(
            "VALUE".to_string(),
            AttributeValue::QuotedString("VAL".to_string()),
        );

        let var = PlayListVariableDefinition::try_from(attrs).unwrap();
        assert_eq!(
            var,
            PlayListVariableDefinition::NameValue {
                name: "VAR".to_string(),
                value: "VAL".to_string(),
            }
        );
    }

    #[test]
    fn test_variable_def_from_attributes_import() {
        let mut attrs = AttributeList::new();
        attrs.insert(
            "IMPORT".to_string(),
            AttributeValue::QuotedString("VAR".to_string()),
        );

        let var = PlayListVariableDefinition::try_from(attrs).unwrap();
        assert_eq!(
            var,
            PlayListVariableDefinition::Import {
                name: "VAR".to_string(),
            }
        );
    }

    #[test]
    fn test_variable_def_from_attributes_query() {
        let mut attrs = AttributeList::new();
        attrs.insert(
            "QUERY".to_string(),
            AttributeValue::QuotedString("VAR".to_string()),
        );

        let var = PlayListVariableDefinition::try_from(attrs).unwrap();
        assert_eq!(
            var,
            PlayListVariableDefinition::QueryParam {
                name: "VAR".to_string(),
                value: String::new(),
            }
        );
    }

    #[test]
    fn test_variable_def_from_attributes_invalid() {
        let mut attrs = AttributeList::new();
        attrs.insert(
            "NAME".to_string(),
            AttributeValue::QuotedString("VAR".to_string()),
        );
        attrs.insert(
            "IMPORT".to_string(),
            AttributeValue::QuotedString("VAR".to_string()),
        );

        assert!(PlayListVariableDefinition::try_from(attrs).is_err());
    }
}
