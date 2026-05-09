use crate::{attribute_list::AttributeList, error::ParseError, playlist::{PlayListVariableDefinition, SharedTag}, segment::Key};

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
    Shared(SharedTag),
    Exclusive(MultivariantExclusiveTag),
}

impl MultivariantPlaylist {
    fn apply_tag(&mut self, tag: SharedTag) -> Result<(), ParseError> {
        match tag {
            SharedTag::Version(v) => {
                if self
                    .tags
                    .iter()
                    .any(|t| matches!(t, MultivariantTag::Shared(SharedTag::Version(_))))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-VERSION")));
                }

                self.tags.push(MultivariantTag::Shared(tag));
            }

            SharedTag::IndependentSegments => {
                if self
                    .tags
                    .iter()
                    .any(|t| matches!(t, MultivariantTag::Shared(SharedTag::IndependentSegments)))
                {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-INDEPENDENT-SEGMENTS",
                    )));
                }

                self.tags.push(MultivariantTag::Shared(tag));
            }

            SharedTag::Variable(v) => match v {
                PlayListVariableDefinition::NameValue { name: _, value: _ } => {}
                PlayListVariableDefinition::Import { name: _ } => {
                    return Err(ParseError::InvalidAttributeDefinition(String::from(
                        "IMPORT attribute MUST not occur in Multivariant playlists",
                    )));
                }

                PlayListVariableDefinition::QueryParam { name: _ } => {
                    return Err(ParseError::InvalidAttributeDefinition(String::from(
                        "QUERYPARAM attribute MUST not occur in Multivariant playlists",
                    )));
                }
            },

            SharedTag::Start {
                precise: _,
                time_offset: _,
            } => {
                if self.tags.iter().any(|t| {
                    matches!(
                        t,
                        MultivariantTag::Shared(SharedTag::Start { precise: _, .. })
                    )
                }) {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-START:PRECISE",
                    )));
                }
                self.tags.push(MultivariantTag::Shared(tag));
            }
        }

        Ok(())
    }
}
