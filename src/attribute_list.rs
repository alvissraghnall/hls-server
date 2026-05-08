use std::{collections::HashMap, str::FromStr};

use crate::{error::ParseError, playlist::PlayListVariableDefinition};

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
