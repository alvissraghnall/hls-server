use crate::{
    attribute_list::AttributeList,
    error::{ParseError, ValidationError},
    multivariant::{MultivariantPlaylist, MultivariantTag},
    playlist::{PlayListVariableDefinition, SharedTag},
};

enum MediaExclusiveTag {
    TargetDuration(u64), // #EXT-X-TARGETDURATION:
    MediaSequence(u64),
    DiscontinuitySequence(u64),
    EndList,
    PlaylistType(PlayListType),
    IFramesOnly,
    PartInf(AttributeList),
    ServerControl(AttributeList),
}

enum PlayListType {
    VOD,
    Event,
}

pub enum MediaTag {
    Shared(SharedTag),
    Exclusive(MediaExclusiveTag),
}

pub struct MediaPlaylist {
    pub tags: Vec<MediaTag>,

    pub variables: Vec<PlayListVariableDefinition>,
}

struct PlaylistContext<'a> {
    uri: &'a str,
    parent_multivariant: Option<&'a MultivariantPlaylist>,
}

impl Default for MediaPlaylist {
    fn default() -> Self {
        Self { tags: Vec::new(), variables: Vec::new() }
    }
}

impl MediaPlaylist {
    fn apply_tag(&mut self, tag: SharedTag) -> Result<(), ParseError> {
        match tag {
            SharedTag::Version(v) => {
                if self
                    .tags
                    .iter()
                    .any(|t| matches!(t, MediaTag::Shared(SharedTag::Version(_))))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-VERSION")));
                }

                self.tags.push(MediaTag::Shared(tag));
            }

            SharedTag::IndependentSegments => {
                if self
                    .tags
                    .iter()
                    .any(|t| matches!(t, MediaTag::Shared(SharedTag::IndependentSegments)))
                {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-INDEPENDENT-SEGMENTS",
                    )));
                }

                self.tags.push(MediaTag::Shared(tag));
            }

            SharedTag::Variable(v) => match v {
                PlayListVariableDefinition::NameValue { name: _, value: _ } => {}
                PlayListVariableDefinition::Import { name: _ } => {}

                PlayListVariableDefinition::QueryParam { name: _ } => {}
            },

            SharedTag::Start {
                precise: _,
                time_offset: _,
            } => {
                if self
                    .tags
                    .iter()
                    .any(|t| matches!(t, MediaTag::Shared(SharedTag::Start { precise: _, .. })))
                {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-START:PRECISE",
                    )));
                }
                self.tags.push(MediaTag::Shared(tag));
            }
        }

        Ok(())
    }

    fn validate(&self, ctx: &PlaylistContext) -> Result<(), ValidationError> {
        let master = ctx.parent_multivariant;
        for tag in &self.tags {
            if let MediaTag::Shared(SharedTag::Variable(v)) = tag {
                match v {
                    PlayListVariableDefinition::Import { .. } => match master {
                        None => {
                            return Err(ValidationError::ImportMediaWithoutMultivariant);
                        }
                        Some(master) => {
                            if !master.tags.iter().any(|t| match t {
                                MultivariantTag::Shared(SharedTag::Variable(
                                    PlayListVariableDefinition::NameValue { name, .. },
                                )) => name == v.get_name(),

                                _ => false,
                            }) {
                                return Err(ValidationError::UnknownImportedVariable(
                                    v.get_name().to_string(),
                                ));
                            }
                        }
                    },
                    PlayListVariableDefinition::NameValue { name, value } => {}
                    PlayListVariableDefinition::QueryParam { name } => {}
                }
            }
        }

        Ok(())
    }
}
