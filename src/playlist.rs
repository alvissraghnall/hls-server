use std::collections::HashMap;

use chrono::{Date, DateTime, FixedOffset};

use crate::{attribute_list::{AttributeList, AttributeValue}, error::ParseError};

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

struct MediaMetadata {
    daterange: DateRange,
    skip: Vec<AttributeList>,
    preload_hint: Vec<AttributeList>,
    rendition_report: Option<AttributeList>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DateRange {
    id: String,
    class: Option<String>,
    start_date: DateTime<FixedOffset>,
    end_date: Option<DateTime<FixedOffset>>,
    duration: Option<f64>,
    planned_duration: Option<f64>,
    end_on_next: bool,
    
    extensions: HashMap<String, AttributeValue>,
}


impl Default for SharedTag {
    fn default() -> Self {
        Self::Version(0)
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
            0 => return Err(ParseError::NoAttribute),
            2.. => return Err(ParseError::TooManyAttributes),
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

        Err(ParseError::NoAttribute)
    }
}

impl TryFrom<AttributeList> for DateRange {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let id = map
            .remove("ID")
            .ok_or(ParseError::InvalidAttributeValue("ID".to_string()))?
            .as_quoted_string()
            .ok_or(ParseError::ExpectedQuotedString)?
            .to_string();

        let class = match map.remove("CLASS") {
            Some(val) => Some(
                val.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)?
                    .to_string(),
            ),
            None => None,
        };

        // let start_date = map.remove("START-DATE")
        //     .ok_or(ParseError::InvalidAttributeValue("START-DATE".to_string()))?
        //     .as_quoted_string()
        //     .

        Err(ParseError::UnknownAttribute("ID".to_string()))
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
