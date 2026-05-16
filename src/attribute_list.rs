use std::{collections::HashMap, str::FromStr};

use crate::{error::ParseError, playlist::PlayListVariableDefinition};

/**
*
  An AttributeValue is one of the following:

  *  decimal-integer: an unquoted string of characters from the set
     [0-9] expressing an integer in base-10 arithmetic in the range
     from 0 to 2^64-1 (18446744073709551615).  A decimal-integer may be
     from 1 to 20 characters long.

  *  hexadecimal-sequence: an unquoted string of characters from the
     set [0-9] and [A-F] that is prefixed with 0x or 0X.  The maximum
     length of a hexadecimal-sequence depends on its AttributeNames.

  *  decimal-floating-point: an unquoted string of characters from the
     set [0-9] and '.' that expresses a non-negative floating-point
     number in decimal positional notation.

  *  signed-decimal-floating-point: an unquoted string of characters
     from the set [0-9], '-', and '.' that expresses a signed floating-
     point number in decimal positional notation.

  *  quoted-string: a string of characters within a pair of double
     quotes (0x22).  The following characters MUST NOT appear in a
     quoted-string: line feed (0xA), carriage return (0xD), or double
     quote (0x22).  The string MUST be non-empty, unless specifically
     allowed.  Quoted-string AttributeValues SHOULD be constructed so
     that byte-wise comparison is sufficient to test two quoted-string
     AttributeValues for equality.  Note that this implies case-
     sensitive comparison.

  *  enumerated-string: an unquoted character string from a set that is
     explicitly defined by the AttributeName.  An enumerated-string
     will never contain double quotes ("), commas (,), or whitespace.

   *  enumerated-string-list: a quoted-string containing a comma-
      separated list of enumerated-strings from a set that is explicitly
      defined by the AttributeName.  Each enumerated-string in the list
      is a string consisting of characters valid in an enumerated-
      string.  The list SHOULD NOT repeat any enumerated-string.  To
      support forward compatibility, clients MUST ignore any
      unrecognized enumerated-strings in an enumerated-string-list.

   *  decimal-resolution: two decimal-integers separated by the "x"
      character.  The first integer is a horizontal pixel dimension
      (width); the second is a vertical pixel dimension (height).
 */
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AttributeValue {
    DecimalInteger(u64),
    DecimalFloatingPoint(f64),
    SignedDecimalFloatingPoint(f64),
    HexSequence(u128),
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

    pub(crate) fn as_hex_sequence(&self) -> Option<u128> {
        match self {
            Self::HexSequence(s) => Some(*s),
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

        "PRECISE" => {
            let parsed = parse_enumerated_string(value)?;
            Ok(AttributeValue::EnumeratedString(parsed))
        }

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
        "CAN-SKIP-DATERANGES" => {
            let parsed = parse_enumerated_string(value)?;
            Ok(AttributeValue::EnumeratedString(parsed))
        }
        "HOLD-BACK" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "PART-HOLD-BACK" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "CAN-BLOCK-RELOAD" => {
            let parsed = parse_enumerated_string(value)?;
            Ok(AttributeValue::EnumeratedString(parsed))
        }
        "METHOD" => {
            let parsed = parse_enumerated_string(value)?;
            Ok(AttributeValue::EnumeratedString(parsed))
        }
        "URI" => {
            let parsed = parse_quoted_string(value)?;
            Ok(AttributeValue::QuotedString(parsed))
        }
        "IV" => Ok(AttributeValue::HexSequence(parse_hex_sequence(value)?)),
        "KEYFORMAT" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),
        "KEYFORMATVERSIONS" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),

        "BYTERANGE" => {
            let value = parse_quoted_string(value)?;
            let parts: Vec<&str> = value.split('@').collect();
            if parts.len() != 2 {
                return Err(ParseError::InvalidAttributeValue(value.to_string()));
            }
            let length = parts[0].parse::<u64>()?;
            let offset = parts[1].parse::<u64>()?;
            Ok(AttributeValue::DecimalInteger(length + offset)) // store the end byte
        }
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

pub(crate) fn parse_enumerated_string(value: &str) -> Result<String, ParseError> {
    if value.contains('"') || value.contains(',') || value.contains(char::is_whitespace) {
        return Err(ParseError::InvalidEnumeratedString(value.to_string()));
    }
    Ok(value.to_string())
}

// nid a secomd look
pub(crate) fn parse_decimal_resolution(value: &str) -> Result<(u64, u64), ParseError> {
    let parts: Vec<&str> = value.split('x').collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidDecimalResolution(value.to_string()));
    }
    let width = parts[0].parse()?;
    let height = parts[1].parse()?;
    Ok((width, height))
}

pub(crate) fn parse_hex_sequence(value: &str) -> Result<u128, ParseError> {
    let parsed = value.trim_start_matches("0x");
    Ok(u128::from_str_radix(parsed, 16)?)
}
