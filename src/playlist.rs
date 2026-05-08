use crate::{segment::Key, types::AttributeList};

pub enum SharedTag {
    Version(u8),
    Variables(Vec<PlayListVariableDefinition>),
    IndependentSegments,
    Start { precise: bool, time_offset: f64 },
}

pub(crate) enum PlayListVariableDefinition {
    NameValue { name: String, value: String },
    Import { name: String },
    QueryParam { name: String },
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
