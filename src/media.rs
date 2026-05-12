use crate::{
    attribute_list::AttributeList,
    error::{ParseError, ValidationError},
    multivariant::{MultivariantPlaylist, MultivariantTag},
    playlist::{PlayListVariableDefinition, SharedTag},
    shared::is_valid_ext_x_define as is_valid_quoted_string,
    uri::decode_uri,
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
        Self {
            tags: Vec::new(),
            variables: Vec::new(),
        }
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
                PlayListVariableDefinition::NameValue { name: _, value: _ }
                | PlayListVariableDefinition::QueryParam { name: _, value: _ }
                | PlayListVariableDefinition::Import { name: _ } => {
                    if self.variables.iter().any(|v| {
                        matches!(
                            v,
                            PlayListVariableDefinition::NameValue { name, value: _ }
                                | PlayListVariableDefinition::QueryParam { name, value: _ }
                                | PlayListVariableDefinition::Import { name }
                                if name == v.get_name()
                        )
                    }) {
                        return Err(ParseError::DuplicateTag(String::from("EXT-X-DEFINE")));
                    }
                    self.variables.push(v);
                }
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

    fn validate(&mut self, ctx: &PlaylistContext) -> Result<(), ValidationError> {
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
                    PlayListVariableDefinition::QueryParam { name, value: _ } => {
                        let decoded = decode_uri(ctx.uri);

                        // verify the decoded URI contains the name as a query param
                        if !is_valid_quoted_string(&decoded) || !decoded.contains(name) {
                            return Err(ValidationError::UnknownImportedVariable(
                                decoded.to_string(),
                            ));
                        }
                        // we want to check for:::
                        // eg: /path/to/playlist.m3u8?&xx=yy&tt=lola
                        // so, first we split the uri by '?' to get the query params
                        // then we split each param by '=' to get the name/value pair
                        // then we check if the name matches the one we're looking for
                        // if not, we return an error
                        // in code:
                        // url.split('?').last().unwrap_or("") returns "name=value&xx=yy&tt=lola"
                        // then we split by '&' to get the individual params
                        // and check if any of them match the name we're looking for
                        let var = decoded
                            .split('?')
                            .last()
                            .unwrap_or("")
                            .split('&') // curr: "name=value"
                            .find(|param| param.split('=').next() == Some(name));
                        let value = var.unwrap().split('=').nth(1).unwrap_or("");

                        if var.is_none() || value.is_empty() {
                            return Err(ValidationError::UnknownImportedVariable(
                                decoded.to_string(),
                            ));
                        }

                        self.variables.iter_mut()
                            .find(|v| matches!(v, PlayListVariableDefinition::QueryParam { name: nom, value: _ } if var == Some(nom)))
                            .map(|v| {
                                if let PlayListVariableDefinition::QueryParam { name: _, value: _ } = v {
                                    *v = PlayListVariableDefinition::QueryParam { name: name.to_string(), value: value.to_string() };
                                }
                            });
                    }
                }
            }
        }

        Ok(())
    }
}
