use core::fmt;
use std::{default, str::FromStr};

use crate::{
    attribute_list::{
        AttributeList, AttributeValue, is_valid_cpc_label,
        is_valid_ext_x_define as is_valid_quoted_string, parse_attribute_list,
    },
    codecs::{self, Codec, SupplementalCodecEntry, fourcc::Fourcc, parse::parse_codecs_attr},
    error::{ParseError, SupplementalCodecParseError, ValidationError},
    playlist::{PlayListVariableDefinition, SharedTag},
    segment::{Key, Method},
    uri::{Uri, decode},
};

#[derive(Debug, PartialEq)]
pub(crate) struct SessionData {
    data_id: String,
    data_type: SessionDataType,
    format: SessionDataFormat,
    language: Option<String>,
}

#[derive(Debug, PartialEq)]
enum SessionDataFormat {
    Raw,
    Json,
}

struct PlaylistContext<'a> {
    uri: &'a str,
}

#[derive(Debug, PartialEq)]
enum SessionDataType {
    Value(String),
    Uri(Uri),
}

impl Default for MultivariantPlaylist {
    fn default() -> Self {
        Self {
            shared_tags: Vec::new(),
            exclusive_tags: Vec::new(),
            variables: Vec::new(),
        }
    }
}

pub struct MultivariantPlaylist {
    pub(crate) shared_tags: Vec<SharedTag>,
    pub(crate) exclusive_tags: Vec<MultivariantExclusiveTag>,
    pub(crate) variables: Vec<PlayListVariableDefinition>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum MultivariantExclusiveTag {
    Media(Media),
    StreamInf(StreamInf),
    IFrameStreamInf(IFrameStreamInf),
    SessionData(SessionData),
    SessionKey(Key),
    ContentSteering((Uri, Option<String>)), //server-uri / pathway-id
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IFrameStreamInf {
    uri: Uri,
    bandwidth: u64,
    average_bandwidth: Option<u64>,
    score: Option<f64>,
    codecs: Vec<Codec>,
    supplemental_codecs: SupplementalCodecs,
    resolution: Option<(u64, u64)>,
    hdcp_level: Option<HdcpLevel>,
    allowed_cpc: Vec<AllowedCpcEntry>,
    video_range: VideoRange,
    req_video_layout: Option<Vec<ViewPresentationEntry>>,
    stable_variant_id: Option<String>,
    video: Option<String>,
    pathway_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StreamInf {
    bandwidth: u64,
    average_bandwidth: Option<u64>,
    score: Option<f64>,
    codecs: Vec<Codec>,
    supplemental_codecs: SupplementalCodecs,
    resolution: Option<(u64, u64)>,
    frame_rate: Option<f64>,
    hdcp_level: Option<HdcpLevel>,
    allowed_cpc: Vec<AllowedCpcEntry>,
    video_range: VideoRange,
    req_video_layout: Option<Vec<ViewPresentationEntry>>,
    stable_variant_id: Option<String>,
    audio: Option<String>,
    video: Option<String>,
    subtitles: Option<String>,
    closed_captions: Option<String>,
    pathway_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ViewPresentationEntry(Vec<PresentationEntrySpecifier>);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PresentationEntrySpecifier {
    VideoChannelSpecifier(VideoChannelSpecifier),
    ProjectionSpecifier(ProjectionSpecifier),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VideoChannelSpecifier {
    Stereo,
    Mono,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProjectionSpecifier {
    Rect,
    Equi,
    Hequ,
    Prim,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AllowedCpcEntry {
    keyformat: String,
    labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VideoRange {
    Sdr,
    Hlg,
    Pq,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HdcpLevel {
    Type0,
    Type1,
    None,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Media {
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

#[derive(Debug, PartialEq)]
pub enum MultivariantTag {
    Shared(SharedTag),
    Exclusive(MultivariantExclusiveTag),
}

//   "dvh1.08.07/db4h"
//   "dvh1.08.07/db4h,dvh1.05.06"
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SupplementalCodecs {
    pub entries: Vec<SupplementalCodecEntry>,
}

impl SupplementalCodecs {
    pub fn new(entries: Vec<SupplementalCodecEntry>) -> Self {
        Self { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SupplementalCodecEntry> {
        self.entries.iter()
    }

    /// validate that every enhancement codec has its base-layer declared in
    /// `codecs` (the parsed CODECS attribute value). returns the first
    /// offending entry if a base layer is missing.
    ///
    /// currently only enforced for DolbyVision entries since the implied
    /// base-layer FourCC is well-defined.  generic/unknown enhancement codecs
    /// are passed through without validation.
    pub fn validate_base_layers<'a>(
        &'a self,
        codecs: &[Codec],
    ) -> Result<(), &'a SupplementalCodecEntry> {
        for entry in &self.entries {
            if let Codec::DolbyVision(dv) = &entry.codec {
                let needed = dv.base_codec_fourcc();
                let found = codecs.iter().any(|c| match c {
                    Codec::Avc(a) => a.get_fourcc() == needed || needed == Fourcc::new(b"avc1"),
                    Codec::Hevc(h) => h.get_fourcc() == needed || needed == Fourcc::new(b"hvc1"),
                    _ => false,
                });
                if !found {
                    return Err(entry);
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for SupplementalCodecs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for entry in &self.entries {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{entry}")?;
            first = false;
        }
        Ok(())
    }
}

impl FromStr for SupplementalCodecs {
    type Err = SupplementalCodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::default());
        }
        let entries = s
            .split(',')
            .enumerate()
            .map(|(i, part)| {
                part.trim().parse::<SupplementalCodecEntry>().map_err(|e| {
                    SupplementalCodecParseError::Entry {
                        index: i,
                        inner: Box::new(e),
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::new(entries))
    }
}

impl MediaType {
    const EXPECTED_STRINGS: [&'static str; 4] = ["AUDIO", "VIDEO", "SUBTITLES", "CLOSED-CAPTIONS"];
}

impl MultivariantPlaylist {
    pub(crate) fn apply_shared_tag(&mut self, tag: SharedTag) -> Result<(), ParseError> {
        match tag {
            SharedTag::Version(v) => {
                if self
                    .shared_tags
                    .iter()
                    .any(|t| matches!(t, SharedTag::Version(_)))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-VERSION")));
                }

                self.shared_tags.push(tag);
            }

            SharedTag::IndependentSegments => {
                if self
                    .shared_tags
                    .iter()
                    .any(|t| matches!(t, SharedTag::IndependentSegments))
                {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-INDEPENDENT-SEGMENTS",
                    )));
                }

                self.shared_tags.push(tag);
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
                    return Err(ParseError::InvalidAttributeDefinition {
                        definition: String::from(
                            "IMPORT attribute MUST not occur in Multivariant playlists",
                        ),
                    });
                }
            },

            SharedTag::Start {
                precise: _,
                time_offset: _,
            } => {
                if self.shared_tags.iter().any(|t| {
                    matches!(
                        t,
                        SharedTag::Start { precise: _, .. }
                    )
                }) {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-START:PRECISE",
                    )));
                }
                self.shared_tags.push(tag);
            }
        }

        Ok(())
    }

    pub(crate) fn apply_exclusive_tag(&mut self, tag: MultivariantExclusiveTag) -> Result<(), ParseError> {
        self.exclusive_tags.push(tag);
        Ok(())
    }

    fn validate(&mut self, ctx: &PlaylistContext) -> Result<(), ValidationError> {
        for tag in &self.shared_tags {
            if let SharedTag::Variable(v) = tag {
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

impl FromStr for MultivariantExclusiveTag {
    type Err = ParseError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_multivariant_exclusive_tag(s)
    }
}

pub(crate) fn parse_multivariant_exclusive_tag(
    line: &str,
) -> Result<MultivariantExclusiveTag, ParseError> {
    match line {
        tag if tag.starts_with("#EXT-X-MEDIA:") => {
            let attr_str = &tag["#EXT-X-MEDIA:".len()..];
            let attrs = parse_attribute_list(attr_str)?;

            let media = Media::try_from(attrs)?;

            Ok(MultivariantExclusiveTag::Media(media))
        }

        // a <URI> gotta come in the very next line after this
        // wonder how we'd parse tthat, yeah?
        tag if tag.starts_with("#EXT-X-STREAM-INF:") => {
            let attr_str = &tag["#EXT-X-STREAM-INF:".len()..];
            let attrs = parse_attribute_list(attr_str)?;

            let stream_inf = StreamInf::try_from(attrs)?;

            Ok(MultivariantExclusiveTag::StreamInf(stream_inf))
        }

        tag if tag.starts_with("#EXT-X-I-FRAME-STREAM-INF:") => {
            let attr_str = &tag["#EXT-X-I-FRAME-STREAM-INF:".len()..];
            let attrs = parse_attribute_list(attr_str)?;

            let stream_inf = IFrameStreamInf::try_from(attrs)?;

            Ok(MultivariantExclusiveTag::IFrameStreamInf(stream_inf))
        }

        tag if tag.starts_with("#EXT-X-SESSION-DATA:") => {
            let attr_str = &tag["#EXT-X-SESSION-DATA:".len()..];
            let attrs = parse_attribute_list(attr_str)?;

            let session_data = SessionData::try_from(attrs)?;
            Ok(MultivariantExclusiveTag::SessionData(session_data))
        }

        tag if tag.starts_with("#EXT-X-SESSION-KEY:") => {
            let attr_str = &tag["#EXT-X-SESSION-KEY:".len()..];
            let attrs = parse_attribute_list(attr_str)?;

            let key = Key::try_from(attrs)?;

            if key.get_method() == &Method::None {
                return Err(ParseError::InvalidAttributeValue {
                    attribute: "METHOD".into(),
                    value: "NONE".into(),
                    expected: "METHOD attribute must not be NONE.".into(),
                });
            }

            Ok(MultivariantExclusiveTag::SessionKey(key))
        }

        // deal with this later bubu
        tag if tag.starts_with("#EXT-X-CONTENT-STEERING:") => {
            let attr_str = &tag["#EXT-X-CONTENT-STEERING:".len()..];
            let mut attrs = parse_attribute_list(attr_str)?;

            let uri = attrs
                .remove("URI")
                .ok_or(ParseError::InvalidAttributeValue {
                    attribute: "URI".into(),
                    value: "NONE".into(),
                    expected: "a valid URI".into(),
                })?
                .as_quoted_string()
                .ok_or(ParseError::ExpectedQuotedString)
                .map(|v| v.parse::<Uri>())??;

            let pathway_id = attrs
                .remove("PATHWAY-ID")
                .map(|v| {
                    v.as_quoted_string()
                        .ok_or(ParseError::ExpectedQuotedString)
                        .map(|x| x.to_string())
                })
                .transpose()?;
            
            Ok(MultivariantExclusiveTag::ContentSteering((uri, pathway_id)))
        }


        _ => Err(ParseError::InvalidLine(line.to_string())),
    }
}

impl TryFrom<AttributeList> for Media {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let attr = map
            .remove("TYPE")
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "TYPE".into(),
                value: "NONE".into(),
                expected: "an enumerated string".into(),
            })?;

        let media_type: MediaType = attr
            .as_enumerated_string()
            .ok_or_else(|| ParseError::InvalidEnumeratedString {
                value: attr.to_string(), // or format!("{attr:?}")
                expected: &MediaType::EXPECTED_STRINGS,
            })?
            .parse()?;

        let uri = map
            .remove("URI")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| x.parse().map_err(|e| ParseError::InvalidUri { source: e }))
            })
            .transpose()?;

        let group_id = map
            .remove("GROUP-ID")
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "GROUP-ID".into(),
                value: "NONE".into(),
                expected: "a valid group ID".into(),
            })?
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
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["YES", "NO"],
                    })
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue {
                                attribute: "DEFAULT".into(),
                                value: x.into(),
                                expected: "YES or NO".into(),
                            })
                        }
                    })
            })
            .transpose()?
            .unwrap_or(false);

        let autoselect = map
            .remove("AUTOSELECT")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["YES", "NO"],
                    })
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue {
                                attribute: "AUTOSELECT".into(),
                                value: x.into(),
                                expected: "YES or NO".into(),
                            })
                        }
                    })
            })
            .transpose()?
            .unwrap_or(false);

        let forced = map
            .remove("FORCED")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["YES", "NO"],
                    })
                    .and_then(|x| {
                        if x == "YES" {
                            Ok(true)
                        } else if x == "NO" {
                            Ok(false)
                        } else {
                            Err(ParseError::InvalidAttributeValue {
                                attribute: "FORCED".into(),
                                value: x.into(),
                                expected: "YES or NO".into(),
                            })
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
                                ParseError::InvalidAttributeValue {
                                    attribute: "INSTREAM-ID".into(),
                                    value: x.into(),
                                    expected: "a valid CC ID".into(),
                                }
                            })
                        } else if x.starts_with("SERVICE") {
                            x[7..].parse::<u8>().map(InStreamId::Service).map_err(|_| {
                                ParseError::InvalidAttributeValue {
                                    attribute: "INSTREAM-ID".into(),
                                    value: x.into(),
                                    expected: "a valid SERVICE ID".into(),
                                }
                            })
                        } else {
                            if !x.is_empty()
                                && x.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
                            {
                                Ok(InStreamId::Other(x.to_string()))
                            } else {
                                Err(ParseError::InvalidAttributeValue {
                                    attribute: "INSTREAM-ID".into(),
                                    value: x.into(),
                                    expected: "a valid INSTREAM-ID".into(),
                                })
                            }
                        }
                    })
            })
            .transpose()?;

        let bit_depth = map
            .remove("BIT-DEPTH")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger {
                        found: v.to_string(),
                    })
                    .map(|x| x as u64)
            })
            .transpose()?;

        let sample_rate = map
            .remove("SAMPLE-RATE")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger {
                        found: v.to_string(),
                    })
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
                                "public.accessibility.describes-music-and-sound" => {
                                    MediaCharacteristic::Public(
                                        PublicMediaCharacteristic::DescribesMusicAndSound,
                                    )
                                }
                                "public.easy-to-read" => MediaCharacteristic::Public(
                                    PublicMediaCharacteristic::EasyToRead,
                                ),
                                "public.accessibility.transcribes-spoken-dialog" => {
                                    MediaCharacteristic::Public(
                                        PublicMediaCharacteristic::TranscribesSpokenDialog,
                                    )
                                }
                                "public.accessibility.describes-video" => {
                                    MediaCharacteristic::Public(
                                        PublicMediaCharacteristic::DescribesVideo,
                                    )
                                }
                                "public.machine-generated" => MediaCharacteristic::Public(
                                    PublicMediaCharacteristic::MachineGenerated,
                                ),

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
                            .ok_or(ParseError::InvalidAttributeValue {
                                attribute: "CHANNELS".into(),
                                value: v.to_string(),
                                expected: "a valid channel count".into(),
                            })
                            .and_then(|x| {
                                x.parse::<u64>()
                                    .map_err(|_| ParseError::InvalidAttributeValue {
                                        attribute: "CHANNELS".into(),
                                        value: v.to_string(),
                                        expected: "a valid channel count".into(),
                                    })
                            });

                        let coding_identifiers = x
                            .next()
                            .map(|s| s.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        let special_usage_identifiers = x
                            .next()
                            .map(|s| {
                                s.split(',')
                                    .map(|s| match s {
                                        "BINAURAL" => Ok(SpecialUsageIdentifier::Binaural),
                                        "IMMERSIVE" => Ok(SpecialUsageIdentifier::Immersive),
                                        "DOWNMIX" => Ok(SpecialUsageIdentifier::Downmix),
                                        s if s.starts_with("BED") => s[4..]
                                            .parse::<u8>()
                                            .map(|x| Ok(SpecialUsageIdentifier::Bed(x)))
                                            .map_err(|_| ParseError::InvalidAttributeValue {
                                                attribute: "CHANNELS".into(),
                                                value: v.to_string(),
                                                expected: "a valid bed identifier".into(),
                                            })?,
                                        s if s.starts_with("DOF") => s[4..]
                                            .parse::<u8>()
                                            .map(|x| {
                                                if x == 3 || x == 6 {
                                                    Ok(SpecialUsageIdentifier::Dof(x))
                                                } else {
                                                    Err(ParseError::InvalidAttributeValue {
                                                        attribute: "CHANNELS".into(),
                                                        value: v.to_string(),
                                                        expected: "a valid DOF identifier".into(),
                                                    })
                                                }
                                            })
                                            .map_err(|_| ParseError::InvalidAttributeValue {
                                                attribute: "CHANNELS".into(),
                                                value: v.to_string(),
                                                expected: "a valid DOF identifier".into(),
                                            })?,
                                        s => Ok(SpecialUsageIdentifier::Unknown(s.to_string())),
                                    })
                                    .collect::<Result<Vec<_>, ParseError>>()
                            })
                            .transpose()?
                            .unwrap_or_default();

                        Ok::<Channels, ParseError>(Channels {
                            count: count?,
                            coding_identifiers,
                            special_usage_identifiers,
                        })
                    })
            })
            .transpose()?
            .transpose()?;

        Ok(Media {
            media_type,
            uri,
            assoc_language,
            group_id,
            language,
            name,
            stable_rendition_id,
            default,
            autoselect,
            forced,
            instream_id,
            bit_depth,
            sample_rate,
            characteristics,
            channels,
        })
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
            _ => Err(ParseError::InvalidEnumeratedString {
                value: s.into(),
                expected: &MediaType::EXPECTED_STRINGS,
            }),
        }
    }
}

impl TryFrom<AttributeList> for StreamInf {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let bandwidth = map
            .remove("BANDWIDTH")
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "BANDWIDTH".into(),
                value: "NONE".into(),
                expected: "a valid bandwidth".into(),
            })?
            .as_decimal_integer()
            .ok_or(ParseError::ExpectedDecimalInteger {
                found: "NONE".into(),
            })?;

        let average_bandwidth = map
            .remove("AVERAGE-BANDWIDTH")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger {
                        found: v.to_string(),
                    })
            })
            .transpose()?;

        let score = map
            .remove("SCORE")
            .map(|v| {
                v.as_decimal_floating_point()
                    .and_then(|x| {
                        if x > 0.0 {
                            return Some(x);
                        } else {
                            return None;
                        }
                    })
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "SCORE".into(),
                        value: v.to_string(),
                        expected: "a positive decimal-floating-point score".into(),
                    })
            })
            .transpose()?;

        let codecs = map
            .remove("CODECS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        let mut codec = parse_codecs_attr(x);

                        match codec {
                            Ok(c) => Ok(c),
                            Err(e) => Err(e.1.into()),
                        }
                    })
            })
            .transpose()?
            .unwrap_or_default();

        let supplemental_codecs = map
            .remove("SUPPLEMENTAL-CODECS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| x.parse::<SupplementalCodecs>().map_err(|e| e.into()))
            })
            .transpose()?
            .unwrap_or_default();

        let resolution = map
            .remove("RESOLUTION")
            .map(|v| {
                v.as_decimal_resolution()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "RESOLUTION".into(),
                        value: v.to_string(),
                        expected: "a valid resolution".into(),
                    })
            })
            .transpose()?;

        let frame_rate = map
            .remove("FRAME-RATE")
            .map(|v| {
                v.as_decimal_floating_point()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "FRAME-RATE".into(),
                        value: v.to_string(),
                        expected: "a valid frame rate".into(),
                    })
            })
            .transpose()?;

        let hdcp_level = map
            .remove("HDCP-LEVEL")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["TYPE-0", "TYPE-1", "NONE"],
                    })
                    .and_then(|x| x.parse::<HdcpLevel>())
            })
            .transpose()?;

        let allowed_cpc = map
            .remove("ALLOWED-CPC")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        x.split(',')
                            .map(|s| {
                                let mut s = s.splitn(2, ':');

                                let keyformat = s
                                    .next()
                                    .ok_or(ParseError::InvalidAttributeValue {
                                        attribute: "ALLOWED-CPC".into(),
                                        value: v.to_string(),
                                        expected: "a valid KEYFORMAT attribute value".into(),
                                    })?
                                    .to_string();

                                let labels = s
                                    .next()
                                    .ok_or(ParseError::InvalidAttributeValue {
                                        attribute: "ALLOWED-CPC".into(),
                                        value: v.to_string(),
                                        expected: "a valid label".into(),
                                    })?
                                    .split('/')
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>();
                                Ok(AllowedCpcEntry { keyformat, labels })
                            })
                            .collect::<Result<Vec<_>, ParseError>>()
                    })
            })
            .transpose()?
            .unwrap_or_default();

        let video_range = map
            .remove("VIDEO-RANGE")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["SDR", "HLG", "PQ"],
                    })
                    .and_then(|x| x.parse::<VideoRange>())
            })
            .transpose()?
            .unwrap_or_default();

        let req_video_layout = map
            .remove("REQ-VIDEO-LAYOUT")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        x.split(',')
                            .map(|s| s.parse::<ViewPresentationEntry>())
                            .collect::<Result<Vec<_>, ParseError>>()
                    })
            })
            .transpose()?;

        let stable_variant_id = map
            .remove("STABLE-VARIANT-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let audio = map
            .remove("AUDIO")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let video = map
            .remove("VIDEO")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let subtitles = map
            .remove("SUBTITLES")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let closed_captions = map
            .remove("CLOSED-CAPTIONS")
            .map(|v| {
                v.as_quoted_string()
                    .or_else(|| v.as_enumerated_string())
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "CLOSED-CAPTIONS".into(),
                        value: v.to_string(),
                        expected:
                            "either a quoted-string or an enumerated-string with the value NONE."
                                .into(),
                    })
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let pathway_id = map
            .remove("PATHWAY-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "PATHWAY-ID".into(),
                        value: v.to_string(),
                        expected:
                            "a valid quoted string representing thr PATHWAY-ID attribute value."
                                .into(),
                    })
                    .map(|x| x.to_string())
            })
            .transpose()?;

        Ok(StreamInf {
            bandwidth,
            average_bandwidth,
            score,
            codecs,
            supplemental_codecs,
            resolution,
            frame_rate,
            hdcp_level,
            allowed_cpc,
            video_range,
            req_video_layout,
            stable_variant_id,
            audio,
            video,
            subtitles,
            closed_captions,
            pathway_id,
        })
    }
}

impl TryFrom<AttributeList> for IFrameStreamInf {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let bandwidth = map
            .remove("BANDWIDTH")
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "BANDWIDTH".into(),
                value: "NONE".into(),
                expected: "a valid bandwidth".into(),
            })?
            .as_decimal_integer()
            .ok_or(ParseError::ExpectedDecimalInteger {
                found: "NONE".into(),
            })?;

        let average_bandwidth = map
            .remove("AVERAGE-BANDWIDTH")
            .map(|v| {
                v.as_decimal_integer()
                    .ok_or(ParseError::ExpectedDecimalInteger {
                        found: v.to_string(),
                    })
            })
            .transpose()?;

        let score = map
            .remove("SCORE")
            .map(|v| {
                v.as_decimal_floating_point()
                    .and_then(|x| {
                        if x > 0.0 {
                            return Some(x);
                        } else {
                            return None;
                        }
                    })
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "SCORE".into(),
                        value: v.to_string(),
                        expected: "a positive decimal-floating-point score".into(),
                    })
            })
            .transpose()?;

        let codecs = map
            .remove("CODECS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        let mut codec = parse_codecs_attr(x);

                        match codec {
                            Ok(c) => Ok(c),
                            Err(e) => Err(e.1.into()),
                        }
                    })
            })
            .transpose()?
            .unwrap_or_default();

        let supplemental_codecs = map
            .remove("SUPPLEMENTAL-CODECS")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| x.parse::<SupplementalCodecs>().map_err(|e| e.into()))
            })
            .transpose()?
            .unwrap_or_default();

        let resolution = map
            .remove("RESOLUTION")
            .map(|v| {
                v.as_decimal_resolution()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "RESOLUTION".into(),
                        value: v.to_string(),
                        expected: "a valid resolution".into(),
                    })
            })
            .transpose()?;

        let hdcp_level = map
            .remove("HDCP-LEVEL")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["TYPE-0", "TYPE-1", "NONE"],
                    })
                    .and_then(|x| x.parse::<HdcpLevel>())
            })
            .transpose()?;

        let allowed_cpc = map
            .remove("ALLOWED-CPC")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        x.split(',')
                            .map(|s| {
                                let mut s = s.splitn(2, ':');

                                let keyformat = s
                                    .next()
                                    .ok_or(ParseError::InvalidAttributeValue {
                                        attribute: "ALLOWED-CPC".into(),
                                        value: v.to_string(),
                                        expected: "a valid KEYFORMAT attribute value".into(),
                                    })?
                                    .to_string();

                                let labels = s
                                    .next()
                                    .ok_or(ParseError::InvalidAttributeValue {
                                        attribute: "ALLOWED-CPC".into(),
                                        value: v.to_string(),
                                        expected: "a valid label".into(),
                                    })?
                                    .split('/')
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>();
                                Ok(AllowedCpcEntry { keyformat, labels })
                            })
                            .collect::<Result<Vec<_>, ParseError>>()
                    })
            })
            .transpose()?
            .unwrap_or_default();

        let video_range = map
            .remove("VIDEO-RANGE")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["SDR", "HLG", "PQ"],
                    })
                    .and_then(|x| x.parse::<VideoRange>())
            })
            .transpose()?
            .unwrap_or_default();

        let req_video_layout = map
            .remove("REQ-VIDEO-LAYOUT")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| {
                        x.split(',')
                            .map(|s| s.parse::<ViewPresentationEntry>())
                            .collect::<Result<Vec<_>, ParseError>>()
                    })
            })
            .transpose()?;

        let stable_variant_id = map
            .remove("STABLE-VARIANT-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let video = map
            .remove("VIDEO")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let pathway_id = map
            .remove("PATHWAY-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::InvalidAttributeValue {
                        attribute: "PATHWAY-ID".into(),
                        value: v.to_string(),
                        expected:
                            "a valid quoted string representing thr PATHWAY-ID attribute value."
                                .into(),
                    })
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let uri = map
            .remove("URI")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .and_then(|x| x.parse().map_err(|e| ParseError::InvalidUri { source: e }))
            })
            .transpose()?
            .ok_or_else(|| ParseError::MissingAttribute {
                attribute: "URI".into(),
            })?;

        Ok(IFrameStreamInf {
            uri,
            bandwidth,
            average_bandwidth,
            score,
            codecs,
            supplemental_codecs,
            resolution,
            hdcp_level,
            allowed_cpc,
            video_range,
            req_video_layout,
            stable_variant_id,
            video,
            pathway_id,
        })
    }
}

impl Default for VideoRange {
    fn default() -> Self {
        Self::Sdr
    }
}

impl FromStr for VideoRange {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SDR" => Ok(Self::Sdr),
            "HLG" => Ok(Self::Hlg),
            "PQ" => Ok(Self::Pq),
            _ => Err(ParseError::InvalidAttributeValue {
                attribute: "VIDEO-RANGE".into(),
                value: s.to_string(),
                expected: "a valid video range".into(),
            }),
        }
    }
}

impl FromStr for ViewPresentationEntry {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split('/')
            .map(|s| s.parse::<PresentationEntrySpecifier>())
            .collect::<Result<Vec<_>, ParseError>>()
            .map(ViewPresentationEntry)
    }
}

impl FromStr for PresentationEntrySpecifier {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CH-STEREO" => Ok(Self::VideoChannelSpecifier(VideoChannelSpecifier::Stereo)),
            "CH-MONO" => Ok(Self::VideoChannelSpecifier(VideoChannelSpecifier::Mono)),
            "PROJ-RECT" => Ok(Self::ProjectionSpecifier(ProjectionSpecifier::Rect)),
            "PROJ-EQUI" => Ok(Self::ProjectionSpecifier(ProjectionSpecifier::Equi)),
            "PROJ-HEQU" => Ok(Self::ProjectionSpecifier(ProjectionSpecifier::Hequ)),
            "PROJ-PRIM" => Ok(Self::ProjectionSpecifier(ProjectionSpecifier::Prim)),
            _ => Err(ParseError::InvalidAttributeValue {
                attribute: "REQ-VIDEO-LAYOUT".into(),
                value: s.to_string(),
                expected: "a valid presentation entry".into(),
            }),
        }
    }
}

impl FromStr for HdcpLevel {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TYPE-0" => Ok(Self::Type0),
            "TYPE-1" => Ok(Self::Type1),
            "NONE" => Ok(Self::None),
            _ => Err(ParseError::InvalidAttributeValue {
                attribute: "HDCP-LEVEL".into(),
                value: s.to_string(),
                expected: "a valid HDCP level".into(),
            }),
        }
    }
}

impl TryFrom<AttributeList> for SessionData {
    type Error = ParseError;

    fn try_from(mut map: AttributeList) -> Result<Self, Self::Error> {
        let data_id = map
            .remove("DATA-ID")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?
            .ok_or(ParseError::MissingAttribute {
                attribute: "DATA-ID".into(),
            })?;

        let format = map
            .remove("FORMAT")
            .map(|v| {
                v.as_enumerated_string()
                    .ok_or(ParseError::InvalidEnumeratedString {
                        value: v.to_string(),
                        expected: &["JSON", "RAW"],
                    })
                    .and_then(|x| x.parse::<SessionDataFormat>())
            })
            .transpose()?
            .unwrap_or_default();

        let language = map
            .remove("LANGUAGE")
            .map(|v| {
                v.as_quoted_string()
                    .ok_or(ParseError::ExpectedQuotedString)
                    .map(|x| x.to_string())
            })
            .transpose()?;

        let has_uri = map.contains_key("URI");
        let has_value = map.contains_key("VALUE");
        let count = (has_uri as u8) + (has_value as u8);

        match count {
            0 => {
                return Err(ParseError::MissingAttribute {
                    attribute: "URI or VALUE".into(),
                });
            }
            2.. => {
                return Err(ParseError::InvalidAttributeDefinition {
                    definition: "EXT-X-SESSION-DATA tag MUST contain either a VALUE or URI attribute, but not both.".into()
                });
            }
            1 => {}
        }

        let data_type = if has_uri {
            let uri: Uri = map
                .remove("URI")
                .map(|v| {
                    v.as_quoted_string()
                        .ok_or(ParseError::ExpectedQuotedString)
                        .and_then(|x| x.parse().map_err(|e| ParseError::InvalidUri { source: e }))
                })
                .transpose()?
                .ok_or_else(|| ParseError::MissingAttribute {
                    attribute: "URI".into(),
                })?;
            SessionDataType::Uri(uri)
        } else if has_value {
            let value = map
                .remove("VALUE")
                .map(|v| {
                    v.as_quoted_string()
                        .ok_or(ParseError::ExpectedQuotedString)
                        .map(|x| x.to_string())
                })
                .transpose()?
                .ok_or_else(|| ParseError::MissingAttribute {
                    attribute: "VALUE".into(),
                })?;
            SessionDataType::Value(value)
        } else {

            return Err(ParseError::MissingAttribute {
                attribute: "URI or VALUE".into(),
            });
        };

        Ok(SessionData {
            data_id,
            format,
            language,
            data_type,
        })

    }
}

impl Default for SessionDataFormat {
    fn default() -> Self {
        Self::Json
    }
}

impl FromStr for SessionDataFormat {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "JSON" => Ok(Self::Json),
            "RAW" => Ok(Self::Raw),
            _ => Err(ParseError::InvalidEnumeratedString {
                value: s.into(),
                expected: &["JSON", "RAW"],
            }),
        }
    }
}

impl MultivariantTag {
    fn shared(&self) -> Option<&SharedTag> {
        match self {
            MultivariantTag::Shared(tag) => Some(tag),
            _ => None,
        }
    }
}

impl Media {
    pub fn get_instream_id(&self) -> Option<&InStreamId> {
        self.instream_id.as_ref()
    }
}

impl InStreamId {
    pub fn is_service(&self) -> bool {
        matches!(self, InStreamId::Service(_))
    }

    pub fn is_cc(&self) -> bool {
        matches!(self, InStreamId::CC(_))
    }

    pub fn is_other(&self) -> bool {
        matches!(self, InStreamId::Other(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::SharedTag;

    #[test]
    fn test_multivariant_playlist_apply_version() {
        let mut playlist = MultivariantPlaylist::default();
        playlist.apply_shared_tag(SharedTag::Version(3)).unwrap();
        assert_eq!(playlist.shared_tags.len(), 1);

        assert!(playlist.apply_shared_tag(SharedTag::Version(4)).is_err());
    }

    #[test]
    fn test_multivariant_playlist_apply_import_fail() {
        let mut playlist = MultivariantPlaylist::default();
        assert!(
            playlist
                .apply_shared_tag(SharedTag::Variable(PlayListVariableDefinition::Import {
                    name: "BAD".to_string()
                }))
                .is_err()
        );
    }

    #[test]
    fn test_multivariant_playlist_apply_define() {
        let mut playlist = MultivariantPlaylist::default();
        playlist
            .apply_shared_tag(SharedTag::Variable(PlayListVariableDefinition::NameValue {
                name: "VAR".to_string(),
                value: "VAL".to_string(),
            }))
            .unwrap();
        assert_eq!(playlist.variables.len(), 1);
    }
}
