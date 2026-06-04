use core::fmt;
use std::fmt::{Display, Formatter};

use crate::playlist::SharedTag;

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
    CodecError(CodecParseError),
    MixedPlaylistTypes,
    NoPendingSegment,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ValidationError {
    UnknownImportedVariable(String),
    ImportMediaWithoutMultivariant,
    InvalidUri,
    InvalidMultivariantAttribute,
    InvalidTargetDuration,
    MissingRequiredTag(String),
    InvalidSharedTag(SharedTag, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriError {
    MissingScheme,
    MissingAuthoritySeparator,
    InvalidFormat,
    InvalidPercentEncoding,
    InvalidUtf8,
}

#[derive(Debug)]
pub(crate) enum PlaylistReadError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    BomPresent,
    InvalidControlCharacter(char),
    Parse(ParseError),
}

impl From<std::io::Error> for PlaylistReadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for PlaylistReadError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Utf8(value)
    }
}

impl From<ParseError> for PlaylistReadError {
    fn from(value: ParseError) -> Self {
        PlaylistReadError::Parse(value)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MixedPlaylistTypes => write!(
                f,
                "Mixed playlist types, e.g. Multivariant tags in a media playlist, or vice versa."
            ),
            ParseError::CodecError(e) => write!(f, "codec: {e}"),
            ParseError::InvalidLine(line) => write!(f, "Invalid line: {}", line),
            ParseError::UnknownTag { tag, span } => {
                write!(f, "Unknown tag: {} at {}:{}", tag, span.line, span.column)
            }
            ParseError::DuplicateTag(tag) => write!(f, "Duplicate tag: {}", tag),
            ParseError::ExpectedQuotedString => write!(f, "Expected quoted string"),
            ParseError::InvalidQuotedString { value, reason } => {
                write!(f, "Invalid quoted string: {value}")
            }
            ParseError::UnknownAttribute(attr) => write!(f, "Unknown attribute: {}", attr),
            ParseError::InvalidAttributeValue {
                attribute,
                value,
                expected,
            } => {
                write!(
                    f,
                    "Invalid attribute value for {attribute}: {value} (expected: {expected})"
                )
            }
            ParseError::InvalidAttributeDefinition { definition } => {
                write!(f, "Invalid attribute definition: {definition}")
            }
            ParseError::InvalidUri { source } => {
                write!(f, "Invalid URI: {source}")
            }
            ParseError::NoPendingSegment => write!(f, "No pending segment"),
            ParseError::MissingAttribute { attribute } => {
                write!(f, "Missing attribute: {}", attribute)
            }
            ParseError::InvalidEnumeratedString { value, expected } => write!(
                f,
                "Invalid enumerated string: {value} (expected one of: {})",
                expected.join(", ")
            ),
            ParseError::InvalidHexSequence { value } => write!(f, "Invalid hex sequence: {value}"),
            ParseError::TooManyAttributes { expected, found } => write!(
                f,
                "Too many attributes than required. Expected: {expected}, found: {found}"
            ),
            ParseError::MediaSequenceAfterSegment => write!(f, "Media sequence after segment"),
            ParseError::ExpectedDecimalInteger { found } => {
                write!(f, "Expected decimal integer, found: {found}")
            }
            ParseError::InvalidDateTime { value } => write!(f, "Invalid DateTime: {value}"),
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
            ValidationError::MissingRequiredTag(t) => write!(f, "Missing Required Tag: {t}"),
            ValidationError::InvalidSharedTag(shared, reason) => {
                write!(f, "Invalid Shared Tag ({}): {reason}", shared.to_string())
            }
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

#[derive(Debug)]
pub enum SupplementalCodecParseError {
    Empty,
    /// The codec portion of an entry failed to parse.
    Codec(CodecParseError),
    /// A compatibility brand is not a valid 4-byte ASCII FourCC.
    InvalidBrand(String),
    /// A specific comma-separated entry failed; carries its index.
    Entry {
        index: usize,
        inner: Box<SupplementalCodecParseError>,
    },
}

impl fmt::Display for SupplementalCodecParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty supplemental codec string"),
            Self::Codec(e) => write!(f, "invalid codec: {e}"),
            Self::InvalidBrand(b) => write!(
                f,
                "invalid compatibility brand: {b:?} (must be 4 ASCII bytes)"
            ),
            Self::Entry { index, inner } => write!(f, "entry {index}: {inner}"),
        }
    }
}

impl std::error::Error for SupplementalCodecParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(e) => Some(e),
            Self::Entry { inner, .. } => Some(inner),
            _ => None,
        }
    }
}

impl From<CodecParseError> for SupplementalCodecParseError {
    fn from(e: CodecParseError) -> Self {
        Self::Codec(e)
    }
}

impl From<SupplementalCodecParseError> for ParseError {
    fn from(value: SupplementalCodecParseError) -> Self {
        match value {
            SupplementalCodecParseError::Codec(e) => ParseError::CodecError(e),
            SupplementalCodecParseError::InvalidBrand(b) => {
                ParseError::CodecError(CodecParseError::InvalidFourcc(b))
            }
            _ => ParseError::CodecError(CodecParseError::Empty),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecParseError {
    Empty,
    MissingField(&'static str),
    InvalidHex {
        field: &'static str,
        inner: std::num::ParseIntError,
    },
    InvalidDecimal {
        field: &'static str,
        inner: std::num::ParseIntError,
    },
    UnknownAvcProfile(u8),
    UnknownHevcProfile(u8),
    UnknownVp9Profile(u8),
    InvalidChroma(u8),
    InvalidFourcc(String),
    WrongHexLength {
        field: &'static str,
        expected: usize,
        got: usize,
    },
}

impl std::fmt::Display for CodecParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty codec string"),
            Self::MissingField(name) => write!(f, "missing required field: {name}"),
            Self::InvalidHex { field, inner } => {
                write!(f, "invalid hex in field '{field}': {inner}")
            }
            Self::InvalidDecimal { field, inner } => {
                write!(f, "invalid decimal in field '{field}': {inner}")
            }
            Self::UnknownAvcProfile(p) => write!(f, "unknown AVC profile IDC 0x{p:02x}"),
            Self::UnknownHevcProfile(p) => write!(f, "unknown HEVC profile IDC {p}"),
            Self::UnknownVp9Profile(p) => write!(f, "unknown VP9 profile {p}"),
            Self::InvalidChroma(c) => write!(f, "invalid VP chroma subsampling value {c}"),
            Self::InvalidFourcc(s) => write!(f, "FourCC must be 4 ASCII bytes, got {s:?}"),
            Self::WrongHexLength {
                field,
                expected,
                got,
            } => write!(f, "field '{field}' must be {expected} hex chars, got {got}"),
        }
    }
}

impl std::error::Error for CodecParseError {}

impl From<CodecParseError> for ParseError {
    fn from(value: CodecParseError) -> Self {
        Self::CodecError(value)
    }
}
