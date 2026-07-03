use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::{
    attribute_list::{
        AttributeList, is_valid_ext_x_define as is_valid_quoted_string, parse_attribute_list,
    },
    error::{ParseError, ValidationError},
    multivariant::{MultivariantPlaylist, MultivariantPlaylistItem},
    playlist::{MediaMetadata, PlayListVariableDefinition, SharedTag},
    segment::MediaSegment,
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

#[derive(Debug, Clone, PartialEq)]
pub enum MediaTag {
    Shared(SharedTag),
    Exclusive(MediaExclusiveTag),
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct MediaPlaylist {
    pub items: Vec<MediaPlaylistItem>,

    pub variables: Vec<PlayListVariableDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MediaPlaylistItem {
    MediaSegment(MediaSegment),
    SharedTag(SharedTag),
    ExclusiveTag(MediaExclusiveTag),
    Metadata(MediaMetadata),
}

struct PlaylistContext<'a> {
    uri: &'a str,
    parent_multivariant: Option<&'a MultivariantPlaylist>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct ServerControl {
    can_skip_until: Option<f64>, // value must be at least 6x target duration
    can_skip_dateranges: Option<bool>, // requires the former
    hold_back: Option<f64>,      // default: 3x target duration
    // >= 2x part target duration (MUST)
    // >= 3x target duration (SHOULD)
    part_hold_back: Option<f64>, // required if EXT-X-PART-INF is present
    can_block_reload: bool,
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
    pub(crate) fn get_segments(&self) -> Vec<&MediaSegment> {
        self.items
            .iter()
            .filter_map(|i| match i {
                MediaPlaylistItem::MediaSegment(segment) => Some(segment),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_variables(&self) -> Vec<&PlayListVariableDefinition> {
        self.variables.iter().collect::<Vec<_>>()
    }

    pub(crate) fn get_metadata(&self) -> Vec<&MediaMetadata> {
        self.items
            .iter()
            .filter_map(|i| match i {
                MediaPlaylistItem::Metadata(md) => Some(md),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_shared_tags(&self) -> Vec<&SharedTag> {
        self.items
            .iter()
            .filter_map(|i| match i {
                MediaPlaylistItem::SharedTag(tag) => Some(tag),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_exclusive_tags(&self) -> Vec<&MediaExclusiveTag> {
        self.items
            .iter()
            .filter_map(|i| match i {
                MediaPlaylistItem::ExclusiveTag(tag) => Some(tag),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn add_segment(&mut self, segment: MediaSegment) {
        self.items.push(MediaPlaylistItem::MediaSegment(segment));
    }

    pub(crate) fn apply_shared_tag(&mut self, tag: SharedTag) -> Result<(), ParseError> {
        match tag {
            SharedTag::Version(_) => {
                if self
                    .items
                    .iter()
                    .any(|t| matches!(t, MediaPlaylistItem::SharedTag(SharedTag::Version(_))))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-VERSION")));
                }

                self.items.push(MediaPlaylistItem::SharedTag(tag));
            }

            SharedTag::IndependentSegments => {
                if self.items.iter().any(|t| {
                    matches!(
                        t,
                        MediaPlaylistItem::SharedTag(SharedTag::IndependentSegments)
                    )
                }) {
                    return Err(ParseError::DuplicateTag(String::from(
                        "EXT-X-INDEPENDENT-SEGMENTS",
                    )));
                }

                self.items.push(MediaPlaylistItem::SharedTag(tag));
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
                    .items
                    .iter()
                    .any(|t| matches!(t, MediaPlaylistItem::SharedTag(SharedTag::Start { .. })))
                {
                    return Err(ParseError::DuplicateTag(String::from("EXT-X-START")));
                }
                self.items.push(MediaPlaylistItem::SharedTag(tag));
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
                self.items.push(MediaPlaylistItem::ExclusiveTag(tag));
            }
            MediaExclusiveTag::ServerControl(attrs) => {
                self.items.push(MediaPlaylistItem::ExclusiveTag(
                    MediaExclusiveTag::ServerControl(attrs),
                ));
            }
            _ => {
                return Err(ParseError::InvalidLine(format!(
                    "{tag:?} is not a valid exclusive tag."
                )));
            }
        }
        Ok(())
    }

    fn validate(&mut self, ctx: &PlaylistContext) -> Result<(), ValidationError> {
        let master = ctx.parent_multivariant;
        let target_duration = self
            .items
            .iter()
            .find_map(|t| match t {
                MediaPlaylistItem::ExclusiveTag(MediaExclusiveTag::TargetDuration(d)) => Some(*d),
                _ => None,
            })
            .ok_or(ValidationError::MissingRequiredTag(
                "#EXT-X-TARGETDURATION".to_string(),
            ))?;

        for item in &self.items {
            if let MediaPlaylistItem::SharedTag(shared_tag) = item
                && let SharedTag::Variable(v) = shared_tag
            {
                match v {
                    PlayListVariableDefinition::Import { .. } => match master {
                        None => {
                            return Err(ValidationError::ImportMediaWithoutMultivariant);
                        }
                        Some(master) => {
                            if !master.items.iter().any(|t| match t {
                                MultivariantPlaylistItem::SharedTag(SharedTag::Variable(
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
                    PlayListVariableDefinition::NameValue { name: _, value: _ } => {}
                    PlayListVariableDefinition::QueryParam { name, value: _ } => {
                        let decoded = decode(ctx.uri)?;

                        if !is_valid_quoted_string(&decoded) {
                            return Err(ValidationError::UnknownImportedVariable(decoded.clone()));
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
                        let param = decoded
                            .split('?')
                            .next_back()
                            .unwrap_or("")
                            .split('&')
                            .find(|param| param.split('=').next() == Some(name.as_str()));

                        let Some(param) = param else {
                            return Err(ValidationError::UnknownImportedVariable(decoded.clone()));
                        };

                        let value = param.split('=').nth(1).unwrap_or("");
                        if value.is_empty() {
                            return Err(ValidationError::UnknownImportedVariable(decoded.clone()));
                        }

                        if let Some(v) = self.variables.iter_mut().find(|v| {
                            matches!(v, PlayListVariableDefinition::QueryParam { name: nom, .. } if nom == name)
                        }) {
                            *v = PlayListVariableDefinition::QueryParam {
                                name: name.clone(),
                                value: value.to_string(),
                            };
                        }
                    }
                }
            }

            // EXT-X-PART-INF is REQUIRED if the playlist contains any EXT-X-PART tag.
            if let MediaPlaylistItem::MediaSegment(seg) = item {
                if seg.get_part().is_some()
                    && !self.items.iter().any(|t| {
                        matches!(
                            t,
                            MediaPlaylistItem::ExclusiveTag(MediaExclusiveTag::PartInf { .. })
                        )
                    })
                {
                    return Err(ValidationError::MissingPartInf);
                }
            }

            if let MediaPlaylistItem::ExclusiveTag(excl_tag) = item {
                if let MediaExclusiveTag::ServerControl(sv) = excl_tag {
                    if let Some(can_skip_until) = sv.can_skip_until {
                        if can_skip_until < (6.0 * target_duration as f64) {
                            return Err(ValidationError::InvalidServerControl(
                                "CAN-SKIP-UNTIL must be at least 6x the target duration"
                                    .to_string(),
                            ));
                        }
                    }

                    if let Some(hold_back) = sv.hold_back {
                        if hold_back < (3.0 * target_duration as f64) {
                            return Err(ValidationError::InvalidServerControl(
                                "HOLD-BACK must be at least 3x the target duration".to_string(),
                            ));
                        }
                    }

                    if let Some(part_hold_back) = sv.part_hold_back {
                        if part_hold_back < (2.0 * target_duration as f64) {
                            return Err(ValidationError::InvalidServerControl(
                                "PART-HOLD-BACK must be at least 2x the target duration"
                                    .to_string(),
                            ));
                        }
                    }

                    if sv.can_skip_until.is_none() && sv.can_skip_dateranges.is_some() {
                        return Err(ValidationError::InvalidServerControl(
                            "CAN-SKIP-DATERANGES requires CAN-SKIP-UNTIL to be present".to_string(),
                        ));
                    }

                    if self.items.iter().any(|t| {
                        matches!(
                            t,
                            MediaPlaylistItem::ExclusiveTag(MediaExclusiveTag::PartInf { .. })
                        )
                    }) && sv.part_hold_back.is_none()
                    {
                        return Err(ValidationError::InvalidServerControl(
                            "PART-HOLD-BACK is required if EXT-X-PART-INF is present".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

impl Display for MediaExclusiveTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaExclusiveTag::TargetDuration(d) => write!(f, "#EXT-X-TARGETDURATION:{d}"),
            MediaExclusiveTag::MediaSequence(nu) => write!(f, "#EXT-X-MEDIA-SEQUENCE:{nu}"),
            MediaExclusiveTag::DiscontinuitySequence(nu) => {
                write!(f, "#EXT-X-DISCONTINUITY-SEQUENCE:{nu}")
            }
            MediaExclusiveTag::EndList => write!(f, "#EXT-X-ENDLIST"),
            MediaExclusiveTag::PlaylistType(typ) => write!(f, "#EXT-X-PLAYLIST-TYPE:{typ}"),
            MediaExclusiveTag::IFramesOnly => write!(f, "#EXT-X-I-FRAMES-ONLY"),
            MediaExclusiveTag::PartInf { part_target } => {
                write!(f, "#EXT-X-PART-INF:PART-TARGET={part_target}")
            }
            MediaExclusiveTag::ServerControl(ctrl) => {
                write!(f, "#EXT-X-SERVER-CONTROL:{ctrl}")
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
                Ok(MediaExclusiveTag::TargetDuration(v))
            } else {
                Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))
            }
        }
        // if this doesnt exist, we assume 0
        s if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") => {
            let media_sequence_number = s
                .strip_prefix("#EXT-X-MEDIA-SEQUENCE:")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);

            Ok(MediaExclusiveTag::MediaSequence(media_sequence_number))
        }
        s if line.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE:") => {
            let discontinuity_sequence_number = s
                .strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:")
                .and_then(|v| v.parse::<u64>().ok());

            Ok(MediaExclusiveTag::DiscontinuitySequence(
                discontinuity_sequence_number.unwrap_or(0),
            ))
        }
        _s if line.starts_with("#EXT-X-ENDLIST") => Ok(MediaExclusiveTag::EndList),
        s if line.starts_with("#EXT-X-PLAYLIST-TYPE:") => {
            let value = s
                .strip_prefix("#EXT-X-PLAYLIST-TYPE:")
                .ok_or(ParseError::InvalidLine(line.into()))?;

            let playlist_type = value.parse::<PlayListType>().map_err(|_| {
                ParseError::InvalidLine(format!("{line} is not valid according to HLS spec."))
            })?;

            Ok(MediaExclusiveTag::PlaylistType(playlist_type))
        }
        _s if line.starts_with("#EXT-X-I-FRAMES-ONLY") => Ok(MediaExclusiveTag::IFramesOnly),
        s if line.starts_with("#EXT-X-PART-INF:") => {
            let attrs = s
                .strip_prefix("#EXT-X-PART-INF:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))?;

            let attrs = parse_attribute_list(attrs)?;
            let part_target = attrs
                .get("PART-TARGET")
                .and_then(super::attribute_list::AttributeValue::as_decimal_floating_point)
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))?;
            Ok(MediaExclusiveTag::PartInf { part_target })
        }
        s if line.starts_with("#EXT-X-SERVER-CONTROL:") => {
            let attrs = s
                .strip_prefix("#EXT-X-SERVER-CONTROL:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
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
            write!(f, "CAN-SKIP-UNTIL={can_skip_until}")?;
        }
        if let Some(can_skip_dateranges) = &self.can_skip_dateranges {
            write!(f, ",CAN-SKIP-DATERANGES={can_skip_dateranges}")?;
        }
        if let Some(hold_back) = &self.hold_back {
            write!(f, ",HOLD-BACK={hold_back}")?;
        }
        if let Some(part_hold_back) = &self.part_hold_back {
            write!(f, ",PART-HOLD-BACK={part_hold_back}")?;
        }
        if self.can_block_reload {
            write!(f, ",CAN-BLOCK-RELOAD")?;
        }
        Ok(())
    }
}

impl TryFrom<AttributeList> for ServerControl {
    type Error = ParseError;

    fn try_from(attrs: AttributeList) -> Result<Self, Self::Error> {
        let can_skip_until = attrs
            .get("CAN-SKIP-UNTIL")
            .and_then(super::attribute_list::AttributeValue::as_decimal_floating_point);
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
            .and_then(super::attribute_list::AttributeValue::as_decimal_floating_point);
        let part_hold_back = attrs
            .get("PART-HOLD-BACK")
            .and_then(super::attribute_list::AttributeValue::as_decimal_floating_point);
        let can_block_reload = attrs
            .get("CAN-BLOCK-RELOAD")
            .and_then(|v| v.as_enumerated_string())
            .is_some_and(|v| v == "YES");
        Ok(Self {
            can_skip_until,
            can_skip_dateranges,
            hold_back,
            part_hold_back,
            can_block_reload,
        })
    }
}

impl Display for MediaPlaylist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in &self.items {
            match item {
                MediaPlaylistItem::MediaSegment(segment) => write!(f, "{segment}")?,
                MediaPlaylistItem::SharedTag(tag) => writeln!(f, "{tag}")?,
                MediaPlaylistItem::ExclusiveTag(tag) => writeln!(f, "{tag}")?,
                MediaPlaylistItem::Metadata(md) => writeln!(f, "{md}")?,
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
    fn test_media_playlist_apply_version() {
        let mut playlist = MediaPlaylist::default();
        playlist.apply_shared_tag(SharedTag::Version(3)).unwrap();
        assert_eq!(playlist.get_shared_tags().len(), 1);

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
        assert_eq!(playlist.get_variables().len(), 1);

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
        playlist
            .items
            .push(MediaPlaylistItem::SharedTag(SharedTag::Variable(
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
