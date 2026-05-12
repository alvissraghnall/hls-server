use crate::{attribute_list::AttributeList, error::ParseError};

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
    daterange: Vec<AttributeList>,
    skip: Vec<AttributeList>,
    preload_hint: Vec<AttributeList>,
    rendition_report: Option<AttributeList>,
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
