use crate::{playlist::SharedTag, segment::Key, types::AttributeList};

pub(crate) struct SessionData {
    data_id: String,
    data_type: SessionDataType,
    format: SessionDataFormat,
    language: Option<String>,
}

enum SessionDataFormat {
    Raw,
    Json,
}

enum SessionDataType {
    Value,
    Uri,
}

impl Default for MultivariantPlaylist {
    fn default() -> Self {
        Self { tags: Vec::new() }
    }
}

pub struct MultivariantPlaylist {
    pub tags: Vec<MultivariantTag>,
}

pub(crate) enum MultivariantExclusiveTag {
    Media(AttributeList),
    StreamInf(AttributeList),
    IFrameStreamInf(AttributeList),
    SessionData(SessionData),
    SessionKey(Key),
    ContentSteering((String, Option<String>)), //server-uri / pathway-id
}

pub enum MultivariantTag {
    Common(SharedTag),
    Exclusive(MultivariantExclusiveTag),
}
