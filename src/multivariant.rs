use crate::{
    attribute_list::AttributeList,
    error::{ParseError, ValidationError},
    playlist::{PlayListVariableDefinition, SharedTag},
    segment::Key,
    shared::is_valid_ext_x_define as is_valid_quoted_string,
    uri::decode_uri,
};

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

struct PlaylistContext<'a> {
    uri: &'a str,
}

enum SessionDataType {
    Value,
    Uri,
}

impl Default for MultivariantPlaylist {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            variables: Vec::new(),
        }
    }
}

pub struct MultivariantPlaylist {
    pub tags: Vec<MultivariantTag>,
    pub variables: Vec<PlayListVariableDefinition>,
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
                PlayListVariableDefinition::NameValue { name: _, value: _ }
                | PlayListVariableDefinition::QueryParam { name: _, value: _ } => {
                    if self.variables.iter().any(|v| {
                        matches!(
                            v,
                            PlayListVariableDefinition::NameValue { name, value: _ }
                                | PlayListVariableDefinition::QueryParam { name, value: _ }
                                if name == v.get_name()
                        )
                    }) {
                        return Err(ParseError::DuplicateTag(String::from("EXT-X-DEFINE")));
                    }
                    self.variables.push(v);
                }
                PlayListVariableDefinition::Import { name: _ } => {
                    return Err(ParseError::InvalidAttributeDefinition(String::from(
                        "IMPORT attribute MUST not occur in Multivariant playlists",
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

    fn validate(&mut self, ctx: &PlaylistContext) -> Result<(), ValidationError> {
        for tag in &self.tags {
            if let MultivariantTag::Shared(SharedTag::Variable(v)) = tag {
                match v {
                    PlayListVariableDefinition::Import { .. } => {
                        return Err(ValidationError::InvalidMultivariantAttribute);
                    }
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
            };
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::SharedTag;

    #[test]
    fn test_multivariant_playlist_apply_version() {
        let mut playlist = MultivariantPlaylist::default();
        playlist.apply_tag(SharedTag::Version(3)).unwrap();
        assert_eq!(playlist.tags.len(), 1);

        assert!(playlist.apply_tag(SharedTag::Version(4)).is_err());
    }

    #[test]
    fn test_multivariant_playlist_apply_import_fail() {
        let mut playlist = MultivariantPlaylist::default();
        assert!(
            playlist
                .apply_tag(SharedTag::Variable(PlayListVariableDefinition::Import {
                    name: "BAD".to_string()
                }))
                .is_err()
        );
    }

    #[test]
    fn test_multivariant_playlist_apply_define() {
        let mut playlist = MultivariantPlaylist::default();
        playlist
            .apply_tag(SharedTag::Variable(PlayListVariableDefinition::NameValue {
                name: "VAR".to_string(),
                value: "VAL".to_string(),
            }))
            .unwrap();
        assert_eq!(playlist.variables.len(), 1);
    }
}
