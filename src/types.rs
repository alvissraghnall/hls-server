use std::collections::HashMap;

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
