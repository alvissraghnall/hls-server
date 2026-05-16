use core::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidLine(String),
    UnknownTag(String),
    DuplicateTag(String),
    ExpectedQuotedString,
    InvalidQuotedString(String),
    UnknownAttribute(String),
    InvalidAttributeValue(String),
    NoAttribute,
    TooManyAttributes,
    InvalidAttributeDefinition(String),
    MediaSequenceAfterSegment,
    InvalidEnumeratedString(String),
    InvalidDecimalResolution(String),
    InvalidHexSequence(String),
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
            ParseError::UnknownTag(tag) => write!(f, "Unknown tag: {}", tag),
            ParseError::DuplicateTag(tag) => write!(f, "Duplicate tag: {}", tag),
            ParseError::ExpectedQuotedString => write!(f, "Expected quoted string"),
            ParseError::InvalidQuotedString(str) => write!(f, "Invalid quoted string: {str}"),
            ParseError::UnknownAttribute(attr) => write!(f, "Unknown attribute: {}", attr),
            ParseError::InvalidAttributeValue(attr) => {
                write!(f, "Invalid attribute value: {}", attr)
            }
            ParseError::InvalidAttributeDefinition(str) => {
                write!(f, "Invalid attribute definition: {str}")
            }
            ParseError::NoAttribute => write!(f, "No attribute"),
            ParseError::InvalidHexSequence(str) => write!(f, "Invalid hex sequence: {str}"),
            ParseError::TooManyAttributes => write!(f, "Too many attributes than required."),
            ParseError::MediaSequenceAfterSegment => write!(f, "Media sequence after segment"),
            ParseError::InvalidEnumeratedString(str) => write!(f, "Invalid enumerated string: {str}"),
            ParseError::InvalidDecimalResolution(str) => write!(f, "Invalid decimal resolution: {str}"),
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

impl std::error::Error for ParseError {}

impl std::error::Error for ValidationError {}

impl From<std::num::ParseIntError> for ParseError {
    fn from(err: std::num::ParseIntError) -> Self {
        ParseError::InvalidAttributeValue(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for ParseError {
    fn from(err: std::num::ParseFloatError) -> Self {
        ParseError::InvalidAttributeValue(err.to_string())
    }
}

impl From<UriError> for ValidationError {
    fn from(_: UriError) -> ValidationError {
        ValidationError::InvalidUri
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