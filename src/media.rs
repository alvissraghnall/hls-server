use std::{fmt, str::FromStr};

use crate::{
    CRLF,
    attribute_list::{
        AttributeList, is_valid_ext_x_define as is_valid_quoted_string, parse_attribute_list,
    },
    error::{ParseError, ValidationError},
    multivariant::{MultivariantPlaylist, MultivariantTag},
    playlist::{PlayListVariableDefinition, SharedTag},
    segment::{ByteRange, MediaSegment},
    uri::decode,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MediaExclusiveTag {
    TargetDuration(u64), // #required
    MediaSequence(u64),
    DiscontinuitySequence(u64),
    EndList,
    PlaylistType(PlayListType),
    IFramesOnly, // requires v4 at least
    PartInf { part_target: f64 },
    ServerControl(ServerControl),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlayListType {
    Vod,
    Event,
}

#[derive(Debug, PartialEq)]
pub enum MediaTag {
    Shared(SharedTag),
    Exclusive(MediaExclusiveTag),
}

pub struct MediaPlaylist {
    pub tags: Vec<MediaTag>,
    pub(crate) segments: Vec<MediaSegment>,

    pub variables: Vec<PlayListVariableDefinition>,

    // track whether we've seen the first segment yet for
    // media sequence number validation
    // (i.e. the first segment should only come after #EXT-X-MEDIA-SEQUENCE)
    seen_first_segment: bool, // hmmmm ???
}

struct PlaylistContext<'a> {
    uri: &'a str,
    parent_multivariant: Option<&'a MultivariantPlaylist>,
}

#[derive(Debug, Clone, PartialEq)]
struct ServerControl {
    can_skip_until: Option<f64>, // value must be at least 6x target duration
    can_skip_dateranges: Option<bool>, // requires the former
    hold_back: Option<f64>,      // default: 3x target duration
    // >= 2x part target duration (MUST)
    // >= 3x target duration (SHOULD)
    part_hold_back: Option<f64>, // required if EXT-X-PART-INF is present
    can_block_reload: bool,
}

impl Default for MediaPlaylist {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            variables: Vec::new(),
            segments: Vec::new(),
            seen_first_segment: false,
        }
    }
}

impl fmt::Display for PlayListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayListType::Vod => write!(f, "VOD"),
            PlayListType::Event => write!(f, "EVENT"),
        }
    }
}

impl FromStr for PlayListType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VOD" => Ok(PlayListType::Vod),
            "EVENT" => Ok(PlayListType::Event),
            _ => Err(ParseError::InvalidLine(format!(
                "{s} is not valid according to HLS spec."
            ))),
        }
    }
}

impl MediaPlaylist {
    fn get_segments(&self) -> &[MediaSegment] {
        &self.segments
    }

    fn add_segment(&mut self, segment: MediaSegment) {
        self.segments.push(segment);
    }

    pub(crate) fn apply_shared_tag(&mut self, tag: SharedTag) -> Result<(), ParseError> {
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
                    .any(|t| matches!(t, MediaTag::Shared(SharedTag::Start { .. })))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-START")));
                }
                self.tags.push(MediaTag::Shared(tag));
            }
        }

        Ok(())
    }

    pub(crate) fn apply_exclusive_tag(&mut self, tag: MediaExclusiveTag) -> Result<(), ParseError> {
        match tag {
            MediaExclusiveTag::TargetDuration(_)
            | MediaExclusiveTag::MediaSequence(_)
            | MediaExclusiveTag::DiscontinuitySequence(_)
            | MediaExclusiveTag::EndList
            | MediaExclusiveTag::PlaylistType(_)
            | MediaExclusiveTag::IFramesOnly
            | MediaExclusiveTag::PartInf { .. } => {
                self.tags.push(MediaTag::Exclusive(tag));
            }
            MediaExclusiveTag::ServerControl(attrs) => {
                self.tags
                    .push(MediaTag::Exclusive(MediaExclusiveTag::ServerControl(attrs)));
            }
            _ => {
                return Err(ParseError::InvalidLine(format!(
                    "{:?} is not a valid exclusive tag.",
                    tag
                )));
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
            }

            if let MediaTag::Exclusive(MediaExclusiveTag::TargetDuration(d)) = tag {
                if self
                    .segments
                    .iter()
                    .any(|s| s.get_duration().round() > *d as i64)
                {
                    return Err(ValidationError::InvalidMultivariantAttribute);
                }
            }
        }

        Ok(())
    }
}

impl ToString for MediaExclusiveTag {
    fn to_string(&self) -> String {
        match self {
            MediaExclusiveTag::TargetDuration(d) => format!("#EXT-X-TARGETDURATION:{}", d),
            MediaExclusiveTag::MediaSequence(nu) => format!("#EXT-X-MEDIA-SEQUENCE:{}", nu),
            MediaExclusiveTag::DiscontinuitySequence(nu) => {
                format!("#EXT-X-DISCONTINUITY-SEQUENCE:{}", nu)
            }
            MediaExclusiveTag::EndList => format!("#EXT-X-ENDLIST"),
            MediaExclusiveTag::PlaylistType(typ) => format!("#EXT-X-PLAYLIST-TYPE:{}", typ),
            MediaExclusiveTag::IFramesOnly => format!("#EXT-X-IFRAMES-ONLY"),
            MediaExclusiveTag::PartInf { part_target } => {
                format!("#EXT-X-PART-INF:PART-TARGET={}", part_target)
            }
            MediaExclusiveTag::ServerControl(ctrl) => {
                format!("#EXT-X-SERVER-CONTROL:{}", ctrl)
            }
        }
    }
}

impl FromStr for MediaExclusiveTag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_media_exclusive_tag(s)
    }
}

pub(crate) fn parse_media_exclusive_tag(line: &str) -> Result<MediaExclusiveTag, ParseError> {
    match line {
        s if s.starts_with("#EXT-X-TARGETDURATION:") => {
            let target_duration = s
                .strip_prefix("#EXT-X-TARGETDURATION:")
                .and_then(|v| v.parse::<u64>().ok());

            if let Some(v) = target_duration
                && v >= 1
            {
                return Ok(MediaExclusiveTag::TargetDuration(v));
            } else {
                return Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )));
            }
        }
        // if this doesnt exist, we assume 0
        s if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") => {
            let media_sequence_number = s
                .strip_prefix("#EXT-X-MEDIA-SEQUENCE:")
                .and_then(|v| v.parse::<u64>().ok());

            if let Some(v) = media_sequence_number {
                return Ok(MediaExclusiveTag::MediaSequence(v));
            } else {
                return Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )));
            }
        }
        s if line.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE:") => {
            let discontinuity_sequence_number = s
                .strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:")
                .and_then(|v| v.parse::<u64>().ok());

            if let Some(v) = discontinuity_sequence_number {
                return Ok(MediaExclusiveTag::DiscontinuitySequence(v));
            } else {
                return Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )));
            }
        }
        s if line.starts_with("#EXT-X-ENDLIST") => {
            return Ok(MediaExclusiveTag::EndList);
        }
        s if line.starts_with("#EXT-X-PLAYLIST-TYPE:") => {
            let playlist_type = s
                .strip_prefix("#EXT-X-PLAYLIST-TYPE:")
                .and_then(|v| v.parse::<PlayListType>().ok());

            if let Some(v) = playlist_type {
                return Ok(MediaExclusiveTag::PlaylistType(v));
            } else {
                return Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )));
            }
        }
        s if line.starts_with("#EXT-X-IFRAMES-ONLY:") => {
            return Ok(MediaExclusiveTag::IFramesOnly);
        }
        s if line.starts_with("#EXT-X-PART-INF:") => {
            let attrs = s
                .strip_prefix("#EXT-X-PART-INF:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{} is not valid according to HLS spec.",
                    line
                )))?;

            let attrs = parse_attribute_list(attrs)?;
            let part_target = attrs
                .get("PART-TARGET")
                .and_then(|v| v.as_decimal_floating_point())
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))?;
            Ok(MediaExclusiveTag::PartInf { part_target })
        }
        s if line.starts_with("#EXT-X-SERVER-CONTROL:") => {
            let attrs = s
                .strip_prefix("#EXT-X-SERVER-CONTROL:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{} is not valid according to HLS spec.",
                    line
                )))?;
            let attrs = parse_attribute_list(attrs)?;

            let server_control = ServerControl::try_from(attrs)?;

            Ok(MediaExclusiveTag::ServerControl(server_control))
        }
        _ => Err(ParseError::InvalidLine(line.to_string())),
    }
}

impl ServerControl {
    pub fn new(attrs: AttributeList) -> Result<Self, ParseError> {
        Self::try_from(attrs)
    }
}

impl fmt::Display for ServerControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(can_skip_until) = &self.can_skip_until {
            write!(f, "CAN-SKIP-UNTIL={}", can_skip_until)?;
        }
        if let Some(can_skip_dateranges) = &self.can_skip_dateranges {
            write!(f, ",CAN-SKIP-DATERANGES={}", can_skip_dateranges)?;
        }
        if let Some(hold_back) = &self.hold_back {
            write!(f, ",HOLD-BACK={}", hold_back)?;
        }
        if let Some(part_hold_back) = &self.part_hold_back {
            write!(f, ",PART-HOLD-BACK={}", part_hold_back)?;
        }
        if self.can_block_reload {
            write!(f, ",CAN-BLOCK-RELOAD")?;
        }
        Ok(())
    }
}

impl Default for ServerControl {
    fn default() -> Self {
        Self {
            can_skip_until: None,
            can_skip_dateranges: None,
            hold_back: None,
            part_hold_back: None,
            can_block_reload: false,
        }
    }
}

impl TryFrom<AttributeList> for ServerControl {
    type Error = ParseError;

    fn try_from(attrs: AttributeList) -> Result<Self, Self::Error> {
        let can_skip_until = attrs
            .get("CAN-SKIP-UNTIL")
            .and_then(|v| v.as_decimal_floating_point());
        let can_skip_dateranges =
            attrs
                .get("CAN-SKIP-DATERANGES")
                .and_then(|v| match v.as_enumerated_string() {
                    Some("YES") => Some(true),
                    Some("NO") => Some(false),
                    _ => None,
                });
        let hold_back = attrs
            .get("HOLD-BACK")
            .and_then(|v| v.as_decimal_floating_point());
        let part_hold_back = attrs
            .get("PART-HOLD-BACK")
            .and_then(|v| v.as_decimal_floating_point());
        let can_block_reload = attrs
            .get("CAN-BLOCK-RELOAD")
            .and_then(|v| v.as_enumerated_string())
            .map(|v| v == "YES")
            .unwrap_or(false);
        Ok(Self {
            can_skip_until,
            can_skip_dateranges,
            hold_back,
            part_hold_back,
            can_block_reload,
        })
    }
}

impl ToString for MediaPlaylist {
    fn to_string(&self) -> String {
        // hmm now i think of it: tags vs segments -- are their original
        // order to be preserved ? can i interchange them ?? hmmmmmmmmmm///
        // 
        unimplemented!()
    }
}

impl ToString for MediaTag {
    fn to_string(&self) -> String {
        match self {
            MediaTag::Shared(shared_tag) => shared_tag.to_string(),
            MediaTag::Exclusive(media_exclusive_tag) => media_exclusive_tag.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::SharedTag;

    #[test]
    fn test_media_playlist_apply_version() {
        let mut playlist = MediaPlaylist::default();
        playlist.apply_shared_tag(SharedTag::Version(3)).unwrap();
        assert_eq!(playlist.tags.len(), 1);

        assert!(playlist.apply_shared_tag(SharedTag::Version(4)).is_err());
    }

    #[test]
    fn test_media_playlist_apply_define() {
        let mut playlist = MediaPlaylist::default();
        playlist
            .apply_shared_tag(SharedTag::Variable(PlayListVariableDefinition::NameValue {
                name: "VAR".to_string(),
                value: "VAL".to_string(),
            }))
            .unwrap();
        assert_eq!(playlist.variables.len(), 1);

        assert!(
            playlist
                .apply_shared_tag(SharedTag::Variable(PlayListVariableDefinition::NameValue {
                    name: "VAR".to_string(),
                    value: "VAL2".to_string(),
                }))
                .is_err()
        );
    }

    #[test]
    fn test_media_playlist_validate_import_fail() {
        let mut playlist = MediaPlaylist::default();
        playlist.tags.push(MediaTag::Shared(SharedTag::Variable(
            PlayListVariableDefinition::Import {
                name: "IMPORT_ME".to_string(),
            },
        )));

        let ctx = PlaylistContext {
            uri: "playlist.m3u8",
            parent_multivariant: None,
        };

        assert!(playlist.validate(&ctx).is_err());
    }
}

/*
 * Meine fav albums, April 2026:::
 * > Mike, Earl Sweatshirt & Surf Gang - POMPEII/UTILITY
 * > Marlon Croft - The Internet Killed The Neighbourhood
 * > Millkzy - Floetry The Extension
 * > NARCY - TO BE AN (ARAB)
 * > Natural Elements - aligNmEnt
 * > MELODOWNZ, Coops - BRON
 * > Your stepdad - supergood
 * > Action Bronson - PLANET FROG (MAY, my error)
 * > Blu x Exile - Time Heals Everything
 * > Rosco P. Coldchain & Nicholas Craven - Play With Something Safe
 */
