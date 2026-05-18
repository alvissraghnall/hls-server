use std::{default, str::FromStr};

use crate::{
    attribute_list::{AttributeList, is_valid_ext_x_define as is_valid_quoted_string},
    error::{ParseError, ValidationError},
    playlist::{PlayListVariableDefinition, SharedTag},
    segment::Key,
    uri::{Uri, decode},
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

struct Media {
    media_type: MediaType,
    uri: Option<Uri>,
    group_id: String,
    language: Option<String>,
    name: Option<String>,
    assoc_language: Option<String>,
    stable_rendition_id: Option<String>,
    default: bool,
    autoselect: bool,
    forced: bool,
    instream_id: Option<InStreamId>,
    bit_depth: Option<u64>,
    sample_rate: Option<u64>,
    characteristics: Vec<MediaCharacteristic>,
    channels: Option<Channels>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaCharacteristic {
    Public(PublicMediaCharacteristic),
    Private(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicMediaCharacteristic {
    AuxiliaryContent,
    TranscribesSpokenDialog,
    DescribesMusicAndSound,
    EasyToRead,
    DescribesVideo,
    MachineGenerated,

}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Channels {
    count: u64,

    pub coding_identifiers: Vec<String>,

    pub special_usage_identifiers: Vec<SpecialUsageIdentifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InStreamId {
    CC(u8),
    Service(u8),

    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpecialUsageIdentifier {
    Binaural,
    Immersive,
    Downmix,
    Bed(u8),
    Dof(u8),

    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MediaType {
    Audio,
    Video,
    Subtitles,
    ClosedCaptions,
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
                        let decoded = decode(ctx.uri)?;

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

impl TryFrom<AttributeList> for Media {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let media_type: MediaType = map
            .remove("TYPE")
            .ok_or(ParseError::InvalidAttributeValue(String::from("TYPE")))?
            .as_enumerated_string()
            .ok_or(ParseError::ExpectedEnumeratedString)?
            .parse()?;

        let uri: Option<Uri> = map
            .remove("URI")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        x.parse()
                            .map_err(|_| ParseError::InvalidAttributeValue("URI".to_string()))
                    })
            })
            .transpose()?;

        let group_id = map
            .remove("GROUP-ID")
            .ok_or(ParseError::InvalidAttributeValue(String::from("GROUP-ID")))?
            .as_quoted_string()
            .ok_or(ParseError::ExpectedQuotedString)?
            .to_string();

        let language = map
            .remove("LANGUAGE")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let assoc_language = map
            .remove("ASSOC-LANGUAGE")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let name = map
            .remove("NAME")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let stable_rendition_id = map
            .remove("STABLE-RENDITION-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let default = map
            .remove("DEFAULT")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::ExpectedEnumeratedString)
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue("DEFAULT".to_string()))
                        }
                    })
            })
            .transpose()?
            .unwrap_or(false);

        let autoselect = map
            .remove("AUTOSELECT")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::ExpectedEnumeratedString)
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue("AUTOSELECT".to_string()))
                        }
                    })
            })
            .transpose()?
            .unwrap_or(false);

        let forced = map
            .remove("FORCED")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::ExpectedEnumeratedString)
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue("FORCED".to_string()))
                        }
                    })
            })
            .transpose()?
            .unwrap_or(false);

        let instream_id = map
            .remove("INSTREAM-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        if x.starts_with("CC") {
                            x[2..].parse::<u8>().map(InStreamId::CC).map_err(|_| {
                                ParseError::InvalidAttributeValue("INSTREAM-ID".to_string())
                            })
                        } else if x.starts_with("SERVICE") {
                            x[7..].parse::<u8>().map(InStreamId::Service).map_err(|_| {
                                ParseError::InvalidAttributeValue("INSTREAM-ID".to_string())
                            })
                        } else {
                            if !x.is_empty() && x.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.') {
                                Ok(InStreamId::Other(x.to_string()))
                            } else {
                                Err(ParseError::InvalidAttributeValue("INSTREAM-ID".to_string()))
                            }
                        }
                    })
            })
            .transpose()?;

        let bit_depth = map
            .remove("BIT-DEPTH")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger)
                    .map(|x| x as u64)
            })
            .transpose()?;

        let sample_rate = map
            .remove("SAMPLE-RATE")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger)
                    .map(|x| x as u64)
            })
            .transpose()?;

        let characteristics = map
            .remove("CHARACTERISTICS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| {
                        x.split(',')
                            .map(|s| match s {
                                "public.accessibility.describes-music-and-sound" => MediaCharacteristic::Public(PublicMediaCharacteristic::DescribesMusicAndSound),
                                "public.easy-to-read" => MediaCharacteristic::Public(PublicMediaCharacteristic::EasyToRead),
                                "public.accessibility.transcribes-spoken-dialog" => MediaCharacteristic::Public(PublicMediaCharacteristic::TranscribesSpokenDialog),
                                "public.accessibility.describes-video" => MediaCharacteristic::Public(PublicMediaCharacteristic::DescribesVideo),
                                "public.machine-generated" => MediaCharacteristic::Public(PublicMediaCharacteristic::MachineGenerated),


                                _ => MediaCharacteristic::Private(s.to_string()),
                            })
                            .collect::<Vec<_>>()
                    })
            })
            .transpose()?
            .unwrap_or_default();

        let channels = map
            .remove("CHANNELS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| {
                        let mut x = x.splitn(3, '/');

                        let count = x
                            .next()
                            .ok_or(ParseError::InvalidAttributeValue("CHANNELS".to_string()))
                            .and_then(|x| {
                                x.parse::<u64>()
                                    .map_err(|_| ParseError::InvalidAttributeValue("CHANNELS".to_string()))
                            });

                        let coding_identifiers = x
                            .next()
                            .map(|s| s.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        let special_usage_identifiers = x
                            .next()
                            .map(|s| {
                                s.split(',')
                                    .map(|s| {
                                        match s {
                                            "BINAURAL" => Ok(SpecialUsageIdentifier::Binaural),
                                            "IMMERSIVE" => Ok(SpecialUsageIdentifier::Immersive),
                                            "DOWNMIX" => Ok(SpecialUsageIdentifier::Downmix),
                                            s if s.starts_with("BED") => s[4..]
                                                .parse::<u8>()
                                                .map(|x| Ok(SpecialUsageIdentifier::Bed(x)))
                                                .map_err(|_| {
                                                    ParseError::InvalidAttributeValue(
                                                        "CHANNELS".to_string(),
                                                    )
                                                }),
                                            s if s.starts_with("DOF") => s[4..]
                                                .parse::<u8>()
                                                .map(|x| Ok(SpecialUsageIdentifier::Dof(x)))
                                                .map_err(|_| {
                                                    ParseError::InvalidAttributeValue(
                                                        "CHANNELS".to_string(),
                                                    )
                                                }),
                                            s => Ok(SpecialUsageIdentifier::Unknown(s.to_string())),
                                        }
                                    })
                                    .collect::<Result<Vec<_>, ParseError>>()
                            })
                            .transpose()?
                            .unwrap_or_default();

                        Ok(Channels {
                            count: count?,
                            coding_identifiers,
                            special_usage_identifiers,
                        })
                    })
                
            })
            .transpose()?;

        Err(ParseError::NoAttribute)
    }
}

impl FromStr for MediaType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AUDIO" => Ok(Self::Audio),
            "VIDEO" => Ok(Self::Video),
            "SUBTITLES" => Ok(Self::Subtitles),
            "CLOSED-CAPTIONS" => Ok(Self::ClosedCaptions),
            _ => Err(ParseError::InvalidEnumeratedString(s.to_string())),
        }
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
