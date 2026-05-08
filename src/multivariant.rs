use crate::{error::ParseError, playlist::SharedTag, segment::Key, attribute_list::AttributeList};

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

            SharedTag::Variables(_) => {}

            SharedTag::Start {
                precise,
                time_offset,
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
