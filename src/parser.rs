use itertools::Itertools;

use crate::{
    error::{self, ParseError},
    media::{self, MediaExclusiveTag, MediaPlaylist, MediaTag, parse_media_exclusive_tag},
    multivariant::{MultivariantExclusiveTag, MultivariantPlaylist},
    parser::PlaylistKind::Media,
    playlist::SharedTag,
    read_write,
    segment::{MediaSegment, ParseSegmentState},
    shared::parse_shared_tag,
};

static EXTINF: &'static str = "#EXTINF";
static EXT_X_BYTERANGE: &'static str = "#EXT-X-BYTERANGE";
static EXT_X_DISCONTINUITY: &'static str = "#EXT-X-DISCONTINUITY";
static EXT_X_KEY: &'static str = "#EXT-X-KEY";
static EXT_X_MAP: &'static str = "#EXT-X-MAP";
static EXT_X_PROGRAM_DATE_TIME: &'static str = "#EXT-X-PROGRAM-DATE-TIME";
static EXT_X_GAP: &'static str = "#EXT-X-GAP";
static EXT_X_BITRATE: &'static str = "#EXT-X-BITRATE";
static EXT_X_PART: &'static str = "#EXT-X-PART";

static MEDIA_SEGMENT_TAGS: [&'static str; 9] = [
    EXTINF,
    EXT_X_BYTERANGE,
    EXT_X_DISCONTINUITY,
    EXT_X_KEY,
    EXT_X_MAP,
    EXT_X_PROGRAM_DATE_TIME,
    EXT_X_GAP,
    EXT_X_BITRATE,
    EXT_X_PART,
];

enum ParsedLine {
    Empty,
    Comment,
    Uri(String),

    M3U,

    MediaSegment,

    SharedTag(SharedTag),

    MediaTag(MediaExclusiveTag),

    MultivariantTag(MultivariantExclusiveTag),
}

enum ParsedTag {
    Shared(SharedTag),
    Media(MediaExclusiveTag),
    Multivariant(MultivariantExclusiveTag),
}

pub enum Playlist {
    Media(MediaPlaylist),
    Multivariant(MultivariantPlaylist),
}

struct PlaylistParser {
    kind: PlaylistKind,

    shared_tags: Vec<SharedTag>,

    media_tags: Vec<MediaExclusiveTag>,
    multivariant_tags: Vec<MultivariantExclusiveTag>,
    uris: Vec<(String, usize)>,

    segments: Vec<MediaSegment>,
}

enum PlaylistKind {
    Unknown,
    Media,
    Multivariant,
}

pub fn parse_file_into_playlist(
    path: impl AsRef<std::path::Path>,
) -> Result<Playlist, error::PlaylistReadError> {
    let content = read_write::read_from_file(path)?;

    let mut parser = PlaylistParser::new();
    let mut parse_segment_state = ParseSegmentState::new();

    let mut line_number = 0;
    for raw_line in content.lines() {
        line_number += 1;

        let line = parser.parse_line(raw_line, line_number, &mut parse_segment_state)?;

        parser.consume(line, line_number, &mut parse_segment_state)?;
    }

    Ok(parser.finish()?)
}

impl PlaylistParser {
    pub(crate) fn new() -> Self {
        Self {
            kind: PlaylistKind::Unknown,
            shared_tags: Vec::new(),
            media_tags: Vec::new(),
            multivariant_tags: Vec::new(),
            uris: Vec::new(),
            segments: Vec::new(),
        }
    }
}

impl PlaylistParser {
    fn parse_line(
        &self,
        line: &str,
        line_number: usize,
        segment_state: &mut ParseSegmentState,
    ) -> Result<ParsedLine, ParseError> {
        let line = line.trim();

        if line_number == 1 {
            if line == "#EXTM3U" {
                return Ok(ParsedLine::M3U);
            } else {
                return Err(ParseError::InvalidLine(
                    "First Line of every Media or Multivariant Playlist must be `#EXTM3U`".into(),
                ));
            }
        }

        if line.is_empty() {
            return Ok(ParsedLine::Empty);
        }

        if !line.starts_with('#') {
            return Ok(ParsedLine::Uri(line.to_string()));
        }

        if !line.starts_with("#EXT") {
            return Ok(ParsedLine::Comment);
        }

        if let Ok(tag) = line.parse::<SharedTag>() {
            return Ok(ParsedLine::SharedTag(tag));
        }

        if let Ok(tag) = line.parse::<MediaExclusiveTag>() {
            return Ok(ParsedLine::MediaTag(tag));
        }

        if let Ok(tag) = line.parse::<MultivariantExclusiveTag>() {
            return Ok(ParsedLine::MultivariantTag(tag));
        }

        if let Ok(_) = segment_state.parse_line(line) {
            return Ok(ParsedLine::MediaSegment);
        }

        Err(ParseError::UnknownTag {
            tag: line.into(),
            span: crate::error::Span {
                line: line_number,
                column: 0,
            }, // change soon x
        })
    }

    fn consume(
        &mut self,
        line: ParsedLine,
        line_number: usize,
        segment_state: &mut ParseSegmentState,
    ) -> Result<(), ParseError> {
        match line {
            ParsedLine::Empty | ParsedLine::Comment | ParsedLine::M3U => {}

            ParsedLine::Uri(uri) => {
                self.consume_uri(uri, line_number, segment_state)?;
            }

            ParsedLine::SharedTag(tag) => {
                self.consume_shared_tag(tag);
            }

            ParsedLine::MediaTag(tag) => {
                self.promote_to_media()?;
                self.media_tags.push(tag);
            }

            ParsedLine::MultivariantTag(tag) => {
                self.promote_to_multivariant()?;
                self.multivariant_tags.push(tag);
            }

            ParsedLine::MediaSegment => {
                self.promote_to_media()?;
            }
        }
        Ok(())
    }

    fn promote_to_multivariant(&mut self) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                self.kind = PlaylistKind::Multivariant;
                Ok(())
            }
            PlaylistKind::Multivariant => Ok(()),
            PlaylistKind::Media => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn promote_to_media(&mut self) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                self.kind = PlaylistKind::Media;
                Ok(())
            }
            PlaylistKind::Media => Ok(()),
            PlaylistKind::Multivariant => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn consume_shared_tag(&mut self, tag: SharedTag) {
        self.shared_tags.push(tag);
    }

    fn consume_media_tag(&mut self, tag: MediaExclusiveTag) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                // self.media_tags.push(tag);
                Ok(())
            }

            PlaylistKind::Media => {
                self.media_tags.push(tag);
                Ok(())
            }

            PlaylistKind::Multivariant => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn consume_multivariant_tag(
        &mut self,
        tag: MultivariantExclusiveTag,
    ) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                // self.multivariant_tags.push(tag);
                Ok(())
            }
            PlaylistKind::Multivariant => {
                self.multivariant_tags.push(tag);
                Ok(())
            }

            PlaylistKind::Media => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn consume_uri(
        &mut self,
        uri: String,
        line_number: usize,
        segment_state: &mut ParseSegmentState,
    ) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                // A standalone URI lowk implies we're in a media playlist
                self.promote_to_media()?;
                self.uris.push((uri, line_number));
            }
            PlaylistKind::Media => {
                let pseg = segment_state
                    .take_pending_segment()
                    .ok_or(ParseError::NoPendingSegment)?;
                let media_segment = pseg.build(uri.as_str().into())?;
                self.segments.push(media_segment);

                // media segment, i think we should store uri's and line number
                // so we could enforce media segment validation - eg tags being for next n occurences
                // of uri until we see that tag again..etc
                self.uris.push((uri, line_number));
            }
            PlaylistKind::Multivariant => {
                // in multivariant playlists, a URI line follows a tag like
                // #EXT-X-STREAM-INF. It points to a sub-playlist.
                self.uris.push((uri, line_number));
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Playlist, ParseError> {
        match self.kind {
            PlaylistKind::Media => {
                let mut media_playlist = MediaPlaylist::default();

                // id prefer to use itertools::zip_longest tbf
                for tag in self.media_tags.into_iter().zip_longest(self.shared_tags) {
                    match tag {
                        itertools::EitherOrBoth::Both(media, shared) => {
                            media_playlist.apply_exclusive_tag(media)?;
                            media_playlist.apply_shared_tag(shared)?;
                        }
                        itertools::EitherOrBoth::Left(media) => {
                            media_playlist.apply_exclusive_tag(media)?;
                        }
                        itertools::EitherOrBoth::Right(shared) => {
                            media_playlist.apply_shared_tag(shared)?;
                        }
                    }
                }

                // combine self.media_tags and self.uris into MediaPlaylist
                Ok(Playlist::Media(media_playlist))
            }
            PlaylistKind::Multivariant => {
                let mut multivariant_playlist = MultivariantPlaylist::default();

                for tag in self
                    .multivariant_tags
                    .into_iter()
                    .zip_longest(self.shared_tags)
                {
                    match tag {
                        itertools::EitherOrBoth::Both(media, shared) => {
                            multivariant_playlist.apply_exclusive_tag(media)?;
                            multivariant_playlist.apply_shared_tag(shared)?;
                        }
                        itertools::EitherOrBoth::Left(media) => {
                            multivariant_playlist.apply_exclusive_tag(media)?;
                        }
                        itertools::EitherOrBoth::Right(shared) => {
                            multivariant_playlist.apply_shared_tag(shared)?;
                        }
                    }
                }

                // combine self.multivariant_tags and self.uris into MultivariantPlaylist
                Ok(Playlist::Multivariant(multivariant_playlist))
            }
            PlaylistKind::Unknown => {
                // handle empty/invalid playlists
                // should pro'lly default to Media for now
                Ok(Playlist::Media(MediaPlaylist::default()))
            }
        }
    }
}

impl Playlist {
    pub(crate) fn as_media (&self) -> Option<&MediaPlaylist> {
        if let Playlist::Media(media) = self {
            Some(media)
        } else {
            None
        }
    }

    pub(crate) fn as_multivariant (&self) -> Option<&MultivariantPlaylist> {
        if let Playlist::Multivariant(multivariant) = self {
            Some(multivariant)
        } else {
            None
        }
    }

}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use super::*;

    static ROOT: &'static str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn simple_media() {
        let playlist_file = Path::new(ROOT)
            .join("examples")
            .join("01-dead-simple-media.m3u8");

        let playlist = parse_file_into_playlist(playlist_file);

        match playlist {
            Ok(p) => {
                assert!(matches!(p, Playlist::Media(_)));

                p.as_media().and_then(|media| {
                    assert!(media.tags.);
                });
                
            }
            Err(e) => panic!("Failed due to: {:?}", e),
        }
    }
}
