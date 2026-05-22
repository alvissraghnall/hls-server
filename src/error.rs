use core::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidLine(String),
    UnknownTag {
        tag: String,
        span: Span,
    },
    DuplicateTag(String),
    ExpectedQuotedString,
    InvalidQuotedString {
        value: String,
        reason: Option<&'static str>,
    },
    UnknownAttribute(String),
    InvalidAttributeValue {
        attribute: String,
        value: String,
        expected: &'static str,
    },

    MissingAttribute {
        attribute: String,
    },

    TooManyAttributes {
        expected: usize,
        found: usize,
    },

    InvalidAttributeDefinition {
        definition: String,
    },

    MediaSequenceAfterSegment,

    InvalidEnumeratedString {
        value: String,
        expected: &'static [&'static str],
    },

    InvalidDecimalResolution {
        value: String,
    },

    InvalidHexSequence {
        value: String,
    },

    InvalidDateTime {
        value: String,
    },

    ExpectedDecimalInteger {
        found: String,
    },
    InvalidUri {
        source: UriError,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ValidationError {
    UnknownImportedVariable(String),
    ImportMediaWithoutMultivariant,
    InvalidUri,
    InvalidMultivariantAttribute,
    InvalidTargetDuration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriError {
    MissingScheme,
    MissingAuthoritySeparator,
    InvalidFormat,
    InvalidPercentEncoding,
    InvalidUtf8,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidLine(line) => write!(f, "Invalid line: {}", line),
            ParseError::UnknownTag { tag, span } => write!(f, "Unknown tag: {} at {}:{}", tag, span.line, span.column),
            ParseError::DuplicateTag(tag) => write!(f, "Duplicate tag: {}", tag),
            ParseError::ExpectedQuotedString => write!(f, "Expected quoted string"),
            ParseError::InvalidQuotedString { value, reason } => {
                write!(f, "Invalid quoted string: {value}")
            }
            ParseError::UnknownAttribute(attr) => write!(f, "Unknown attribute: {}", attr),
            ParseError::InvalidAttributeValue { attribute, value, expected } => {
                write!(f, "Invalid attribute value for {attribute}: {value} (expected: {expected})")
            }
            ParseError::InvalidAttributeDefinition { definition } => {
                write!(f, "Invalid attribute definition: {definition}")
            }
            ParseError::InvalidUri { source } => {
                write!(f, "Invalid URI: {source}")
            }
            ParseError::MissingAttribute { attribute } => write!(f, "Missing attribute: {}", attribute),
            ParseError::InvalidEnumeratedString { value, expected } => write!(f, "Invalid enumerated string: {value} (expected one of: {})", expected.join(", ")),
            ParseError::InvalidHexSequence { value } => write!(f, "Invalid hex sequence: {value}"),
            ParseError::TooManyAttributes {
                expected,
                found,
            } => write!(f, "Too many attributes than required. Expected: {expected}, found: {found}"),
            ParseError::MediaSequenceAfterSegment => write!(f, "Media sequence after segment"),
            ParseError::ExpectedDecimalInteger { found } => write!(f, "Expected decimal integer, found: {found}"),
            ParseError::InvalidDateTime {
                value,
            } => write!(f, "Invalid DateTime: {value}"),
            ParseError::InvalidDecimalResolution { value } => {
                write!(f, "Invalid decimal resolution: {value}")
            }
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::UnknownImportedVariable(var) => {
                write!(f, "Unknown imported variable: {var}")
            }
            ValidationError::ImportMediaWithoutMultivariant => {
                write!(f, "Import media playlist without multivariant")
            }
            ValidationError::InvalidMultivariantAttribute => {
                write!(f, "Invalid multivariant attribute")
            }
            ValidationError::InvalidTargetDuration => write!(f, "Invalid target duration"),
            ValidationError::InvalidUri => write!(f, "Invalid URI"),
        }
    }
}

impl From<chrono::ParseError> for ParseError {
    fn from(value: chrono::ParseError) -> Self {
        match value.kind() {
            _ => ParseError::InvalidDateTime {
                value: value.to_string(),
            },
        }
    }
}

impl std::error::Error for ParseError {}

impl std::error::Error for ValidationError {}

impl From<std::num::ParseIntError> for ParseError {
    fn from(err: std::num::ParseIntError) -> Self {
        ParseError::InvalidAttributeValue {
            attribute: "integer".into(),
            value: err.to_string(),
            expected: "a valid integer".into(),
        }
    }
}

impl From<std::num::ParseFloatError> for ParseError {
    fn from(err: std::num::ParseFloatError) -> Self {
        ParseError::InvalidAttributeValue {
            attribute: "float".into(),
            value: err.to_string(),
            expected: "a valid float".into(),
        }
    }
}

impl From<UriError> for ValidationError {
    fn from(_: UriError) -> ValidationError {
        ValidationError::InvalidUri
    }
}

impl From<UriError> for ParseError {
    fn from(source: UriError) -> ParseError {
        ParseError::InvalidUri { source }
    }
}

impl Display for UriError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UriError::MissingScheme => write!(f, "URI missing scheme"),
            UriError::MissingAuthoritySeparator => {
                write!(f, "URI missing '//' after scheme")
            }
            UriError::InvalidFormat => write!(f, "URI has invalid format"),
            UriError::InvalidPercentEncoding => write!(f, "Invalid percent-encoded sequence"),
            UriError::InvalidUtf8 => write!(f, "Decoded bytes are not valid UTF-8"),
        }
    }
}

impl std::error::Error for UriError {}
