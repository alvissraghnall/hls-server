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
}

pub(crate) enum ValidationError {
    UnknownImportedVariable(String),
    ImportMediaWithoutMultivariant,
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
            ParseError::InvalidAttributeValue(attr) => write!(f, "Invalid attribute value: {}", attr),
            ParseError::InvalidAttributeDefinition(str) => write!(f, "Invalid attribute definition: {str}"),
            ParseError::NoAttribute => write!(f, "No attribute"),
            ParseError::TooManyAttributes => write!(f, "Too many attributes than required."),
        }
    }
}

impl std::error::Error for ParseError {}

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