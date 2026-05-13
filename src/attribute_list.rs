use std::{collections::HashMap, str::FromStr};

use crate::{error::ParseError, playlist::PlayListVariableDefinition};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AttributeValue {
    DecimalInteger(u64),
    DecimalFloatingPoint(f64),
    SignedDecimalFloatingPoint(f64),
    HexSequence(String),
    QuotedString(String),
    EnumeratedString(String),
    EnumeratedStringList(Vec<String>),
    DecimalResolution { width: u64, height: u64 },
}

// The actual data structure
pub type AttributeList = HashMap<String, AttributeValue>;

impl ToString for AttributeValue {
    fn to_string(&self) -> String {
        match self {
            Self::SignedDecimalFloatingPoint(x) => x.to_string(),
            Self::DecimalInteger(x) => x.to_string(),
            Self::DecimalFloatingPoint(x) => x.to_string(),
            Self::HexSequence(x) => x.to_string(),
            Self::QuotedString(x) => x.to_string(),
            Self::EnumeratedString(x) => x.to_string(),
            Self::EnumeratedStringList(x) => x.join(",").to_string(),
            Self::DecimalResolution { width, height } => format!("{}x{}", width, height),
        }
    }
}

impl AttributeValue {
    pub(crate) fn as_quoted_string(&self) -> Option<&str> {
        match self {
            Self::QuotedString(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_decimal_floating_point(&self) -> Option<f64> {
        match self {
            Self::DecimalFloatingPoint(x) => Some(*x),
            _ => None,
        }
    }

    pub(crate) fn as_signed_decimal_floating_point(&self) -> Option<f64> {
        match self {
            Self::SignedDecimalFloatingPoint(x) => Some(*x),
            _ => None,
        }
    }

    pub(crate) fn as_enumerated_string(&self) -> Option<&str> {
        match self {
            Self::EnumeratedString(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_enumerated_string_list(&self) -> Option<&[String]> {
        match self {
            Self::EnumeratedStringList(list) => Some(list),
            _ => None,
        }
    }
}

fn parse_attribute_value(name: &str, value: &str) -> Result<AttributeValue, ParseError> {
    match name {
        "BANDWIDTH" => Ok(AttributeValue::DecimalInteger(value.parse()?)),

        "TIME-OFFSET" => Ok(AttributeValue::SignedDecimalFloatingPoint(value.parse()?)),

        "PRECISE" => Ok(AttributeValue::EnumeratedString(value.to_string())),

        "CODECS" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),

        "NAME" => {
            let parsed = parse_quoted_string(value)?;
            if !is_valid_ext_x_define(&parsed) {
                return Err(ParseError::InvalidAttributeValue(value.to_string()));
            }
            Ok(AttributeValue::QuotedString(parsed))
        }
        "VALUE" => {
            let parsed = parse_quoted_string(value)?;

            if !is_valid_ext_x_define_allow_empty(&parsed) {
                return Err(ParseError::InvalidAttributeValue(value.to_string()));
            }

            Ok(AttributeValue::QuotedString(parsed))
        }

        "IMPORT" => {
            let parsed = parse_quoted_string(value)?;
            if !is_valid_ext_x_define(&parsed) {
                return Err(ParseError::InvalidAttributeValue(value.to_string()));
            }
            Ok(AttributeValue::QuotedString(parsed))
        }

        "QUERYPARAM" => {
            let parsed = parse_quoted_string(value)?;
            if !is_valid_ext_x_define(&parsed) {
                return Err(ParseError::InvalidAttributeValue(value.to_string()));
            }
            Ok(AttributeValue::QuotedString(parsed))
        }

        "PART-TARGET" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),

        "CAN-SKIP-UNTIL" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "CAN-SKIP-DATERANGES" => Ok(AttributeValue::EnumeratedString(value.to_string())),

        "HOLD-BACK" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "PART-HOLD-BACK" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "CAN-BLOCK-RELOAD" => Ok(AttributeValue::EnumeratedString(value.to_string())),
        
        _ => Err(ParseError::UnknownAttribute(name.into())),
    }
}

pub(crate) fn parse_attribute_list(s: &str) -> Result<AttributeList, ParseError> {
    let mut attrs = AttributeList::new();
    for (key, value) in s
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.split_once('='))
        .flatten()
    {
        let key = key.trim();
        let value = value.trim();
        attrs.insert(key.to_string(), parse_attribute_value(key, value)?);
    }
    Ok(attrs)
}

pub(crate) fn is_valid_ext_x_define(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub(crate) fn is_valid_ext_x_define_allow_empty(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub(crate) fn parse_quoted_string(value: &str) -> Result<String, ParseError> {
    if !(value.starts_with('"') && value.ends_with('"')) {
        return Err(ParseError::ExpectedQuotedString);
    }

    let inner = &value[1..value.len() - 1];

    if inner.contains('"') || inner.contains('\n') || inner.contains('\r') {
        return Err(ParseError::InvalidQuotedString(inner.to_string()));
    }

    Ok(inner.to_string())
}
