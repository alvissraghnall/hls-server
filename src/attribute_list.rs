use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

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
    HexSequence(Vec<u8>),
    QuotedString(String),
    EnumeratedString(String),
    EnumeratedStringList(HashSet<String>),
    DecimalResolution { width: u64, height: u64 },
    XAttribute(String, Box<AttributeValue>),
}

// The actual data structure
pub type AttributeList = HashMap<String, AttributeValue>;

impl ToString for AttributeValue {
    fn to_string(&self) -> String {
        match self {
            Self::SignedDecimalFloatingPoint(x) => x.to_string(),
            Self::DecimalInteger(x) => x.to_string(),
            Self::DecimalFloatingPoint(x) => x.to_string(),
            Self::HexSequence(x) => format!("0x{}", hex::encode(x)),
            Self::QuotedString(x) => x.to_string(),
            Self::EnumeratedString(x) => x.to_string(),
            // hmmmmmmmmmm [ `clone` ]
            Self::EnumeratedStringList(x) => x.iter().cloned().collect::<Vec<String>>().join(","),
            Self::DecimalResolution { width, height } => format!("{}x{}", width, height),
            Self::XAttribute(name, value) => format!("{}={}", name, value.to_string()),
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

    pub(crate) fn as_hex_sequence(&self) -> Option<&Vec<u8>> {
        match self {
            Self::HexSequence(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_enumerated_string_list(&self) -> Option<&HashSet<String>> {
        match self {
            Self::EnumeratedStringList(list) => Some(list),
            _ => None,
        }
    }

    pub(crate) fn as_decimal_integer(&self) -> Option<u64> {
        match self {
            Self::DecimalInteger(x) => Some(*x),
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
            Ok(AttributeValue::QuotedString(value))
        }

        "DURATION" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "INDEPENDENT" => Ok(AttributeValue::EnumeratedString(parse_enumerated_string(
            value,
        )?)),
        "GAP" => Ok(AttributeValue::EnumeratedString(parse_enumerated_string(
            value,
        )?)),

        "ID" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),
        "CLASS" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),
        "START-DATE" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),
        "END-DATE" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),
        "CUE" => Ok(AttributeValue::EnumeratedStringList(
            parse_enumerated_string_list(value)?,
        )),
        "PLANNED-DURATION" => Ok(AttributeValue::DecimalFloatingPoint(value.parse()?)),
        "END-ON-NEXT" => Ok(AttributeValue::EnumeratedString(parse_enumerated_string(
            value,
        )?)),

        x if x.starts_with("X-") => {
            let parsed = parse_quoted_string(value).map_or_else(
                |e| {
                    parse_hex_sequence(value)
                        .map(|bytes| AttributeValue::HexSequence(bytes))
                        .or_else(|_| {
                            value
                                .parse::<f64>()
                                .map(AttributeValue::SignedDecimalFloatingPoint)
                        })
                },
                |s| Ok(AttributeValue::QuotedString(s)),
            )?;
            Ok(AttributeValue::XAttribute(x.to_string(), Box::new(parsed)))
        }

        "SCTE35-CMD" => Ok(AttributeValue::HexSequence(parse_hex_sequence(value)?)),
        "SCTE35-OUT" => Ok(AttributeValue::HexSequence(parse_hex_sequence(value)?)),
        "SCTE35-IN" => Ok(AttributeValue::HexSequence(parse_hex_sequence(value)?)),

        "SKIPPED-SEGMENTS" => Ok(AttributeValue::DecimalInteger(value.parse()?)),
        "RECENTLY-REMOVED-DATERANGES" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),

        "TYPE" => {
            let parsed = parse_enumerated_string(value)?;
            Ok(AttributeValue::EnumeratedString(parsed))
        }
        "BYTERANGE-START" => Ok(AttributeValue::DecimalInteger(value.parse()?)),
        "BYTERANGE-LENGTH" => Ok(AttributeValue::DecimalInteger(value.parse()?)),
        
        "LAST-MSN" => Ok(AttributeValue::DecimalInteger(value.parse()?)),
        "LAST-PART" => Ok(AttributeValue::DecimalInteger(value.parse()?)),

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

pub(crate) fn parse_enumerated_string_list(value: &str) -> Result<HashSet<String>, ParseError> {
    let items: HashSet<String> = parse_quoted_string(value)?
        .split(',')
        .map(|s| parse_enumerated_string(s))
        .collect::<Result<HashSet<String>, ParseError>>()?;
    Ok(items)
}

// nid a second look
pub(crate) fn parse_decimal_resolution(value: &str) -> Result<(u64, u64), ParseError> {
    let parts: Vec<&str> = value.split('x').collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidDecimalResolution(value.to_string()));
    }
    let width = parts[0].parse()?;
    let height = parts[1].parse()?;
    Ok((width, height))
}

pub(crate) fn parse_hex_sequence(value: &str) -> Result<Vec<u8>, ParseError> {
    if !value.starts_with("0x") {
        return Err(ParseError::InvalidHexSequence(value.to_string()));
    }
    let parsed = value.trim_start_matches("0x");
    let bytes =
        hex::decode(parsed).map_err(|_| ParseError::InvalidHexSequence(String::from(value)))?;
    Ok(bytes)
}