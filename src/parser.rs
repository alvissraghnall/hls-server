use std::{fmt::Display, str::FromStr};

use crate::{
    error::{self, ParseError},
    media::{MediaExclusiveTag, MediaPlaylist},
    multivariant::{
        MultivariantExclusiveTag, MultivariantPlaylist, MultivariantPlaylistItem, PendingStreamInf,
        StreamInfParserState,
    },
    playlist::{MediaMetadata, SharedTag},
    read_write,
    segment::{MediaSegment, ParseSegmentState},
    uri::Uri,
};

static EXTINF: &str = "#EXTINF";
static EXT_X_BYTERANGE: &str = "#EXT-X-BYTERANGE";
static EXT_X_DISCONTINUITY: &str = "#EXT-X-DISCONTINUITY";
static EXT_X_KEY: &str = "#EXT-X-KEY";
static EXT_X_MAP: &str = "#EXT-X-MAP";
static EXT_X_PROGRAM_DATE_TIME: &str = "#EXT-X-PROGRAM-DATE-TIME";
static EXT_X_GAP: &str = "#EXT-X-GAP";
static EXT_X_BITRATE: &str = "#EXT-X-BITRATE";
static EXT_X_PART: &str = "#EXT-X-PART";

static MEDIA_SEGMENT_TAGS: [&str; 9] = [
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

#[derive(Debug, Clone, PartialEq)]
enum ParsedLine {
    Empty,
    Comment,
    Uri(String),

    M3U,

    MediaSegment,

    SharedTag(SharedTag),

    MediaTag(MediaExclusiveTag),

    MultivariantTag(MultivariantExclusiveTag),

    MediaMetadata(MediaMetadata),

    PendingStreamInf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Playlist {
    Media(MediaPlaylist),
    Multivariant(MultivariantPlaylist),
}

enum PlaylistItem {
    SharedTag(SharedTag),

    MediaTag(MediaExclusiveTag),

    Segment(MediaSegment),

    Metadata(MediaMetadata),

    MultivariantTag(MultivariantExclusiveTag),

    Uri(Uri, usize),
}

struct PlaylistParser {
    kind: PlaylistKind,

    items: Vec<PlaylistItem>,

    validator: StructuralValidator,
}

struct StructuralValidator {
    seen_first_media_segment: bool,
    seen_discontinuity: bool,
}

enum PlaylistKind {
    Unknown,
    Media,
    Multivariant,
}

struct ParseContext {
    parse_stream_inf_state: StreamInfParserState,
    parse_segment_state: ParseSegmentState,
    playlist_parser: PlaylistParser,
}

pub fn parse_file_into_playlist(
    path: impl AsRef<std::path::Path>,
) -> Result<Playlist, error::PlaylistReadError> {
    let content = read_write::read_from_file(path)?;

    let mut parse_context = ParseContext::default();

    // for (i, line) in content.lines().enumerate() {
    //     println!("{:>3}: {}", i + 1, line);
    // }

    let mut line_number = 0;
    for raw_line in content.lines() {
        line_number += 1;
        println!("line: {}", raw_line.trim());

        let line = parse_context.parse_line_kind(raw_line, line_number)?;

        parse_context.consume(line, line_number)?;
    }

    Ok(parse_context.finish()?)
}

impl PlaylistParser {
    pub fn new() -> Self {
        Self {
            kind: PlaylistKind::Unknown,
            items: Vec::new(),
            validator: StructuralValidator {
                seen_discontinuity: false,
                seen_first_media_segment: false,
            },
        }
    }
}

impl ParseContext {
    fn parse_line_kind(
        &mut self,
        line: &str,
        line_number: usize,
    ) -> Result<ParsedLine, ParseError> {
        // println!("line: {}", line);

        if line_number == 1 {
            if line == Playlist::EXTM3U {
                return Ok(ParsedLine::M3U);
            }
            return Err(ParseError::InvalidLine(
                "First Line of every Media or Multivariant Playlist must be `#EXTM3U`".into(),
            ));
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
            if let MediaExclusiveTag::MediaSequence(ms) = tag {
                self.parse_segment_state.set_media_sequence(ms);
            }
            return Ok(ParsedLine::MediaTag(tag));
        }

        if let Ok(tag) = line.parse::<MultivariantExclusiveTag>() {
            return Ok(ParsedLine::MultivariantTag(tag));
        }

        if self.parse_segment_state.accept(line).is_ok() {
            println!("media segment: {:?}", line);
            return Ok(ParsedLine::MediaSegment);
        }

        if let Ok(tag) = crate::playlist::MediaMetadata::parse_line(line, line_number) {
            return Ok(ParsedLine::MediaMetadata(tag));
        }

        if self.parse_stream_inf_state.accept_line(line)? {
            println!("pending stream inf: {:?}", line);
            return Ok(ParsedLine::PendingStreamInf);
        }

        Err(ParseError::UnknownTag {
            tag: line.into(),
            span: crate::error::Span {
                line: line_number,
                column: 0,
            }, // change soon x
        })
    }

    fn consume(&mut self, line: ParsedLine, line_number: usize) -> Result<(), ParseError> {
        let line_cl = line.clone();

        match line {
            ParsedLine::Empty | ParsedLine::Comment | ParsedLine::M3U => {}

            ParsedLine::Uri(uri) => {
                self.consume_uri(uri, line_number)?;
            }

            ParsedLine::SharedTag(tag) => {
                self.consume_shared_tag(tag);
            }

            ParsedLine::MediaTag(tag) => {
                self.promote_to_media()?;
                self.consume_media_tag(tag)?;
            }

            ParsedLine::MultivariantTag(tag) => {
                println!(
                    "Tag: {:?}, line: {:?}, line_number: {}",
                    tag, line_cl, line_number
                );
                self.promote_to_multivariant()?;

                self.consume_multivariant_tag(tag)?;
            }

            ParsedLine::MediaSegment => {
                self.promote_to_media()?;
            }

            ParsedLine::MediaMetadata(tag) => {
                self.playlist_parser.items.push(PlaylistItem::Metadata(tag));
            }

            ParsedLine::PendingStreamInf => {
                self.promote_to_multivariant()?;
            }
        }
        Ok(())
    }

    fn promote_to_multivariant(&mut self) -> Result<(), ParseError> {
        match self.playlist_parser.kind {
            PlaylistKind::Unknown => {
                self.playlist_parser.kind = PlaylistKind::Multivariant;
                Ok(())
            }
            PlaylistKind::Multivariant => Ok(()),
            PlaylistKind::Media => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn promote_to_media(&mut self) -> Result<(), ParseError> {
        match self.playlist_parser.kind {
            PlaylistKind::Unknown => {
                self.playlist_parser.kind = PlaylistKind::Media;
                Ok(())
            }
            PlaylistKind::Media => Ok(()),
            PlaylistKind::Multivariant => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn consume_shared_tag(&mut self, tag: SharedTag) {
        self.playlist_parser
            .items
            .push(PlaylistItem::SharedTag(tag));
    }

    fn consume_media_tag(&mut self, tag: MediaExclusiveTag) -> Result<(), ParseError> {
        self.playlist_parser.validator.consume_media_tag(&tag)?;

        self.playlist_parser.items.push(PlaylistItem::MediaTag(tag));

        Ok(())
    }

    fn consume_multivariant_tag(
        &mut self,
        tag: MultivariantExclusiveTag,
    ) -> Result<(), ParseError> {
        match self.playlist_parser.kind {
            PlaylistKind::Unknown => {
                // self.multivariant_tags.push(tag);
                // self.items.push(PlaylistItem::MultivariantTag(tag));
                Ok(())
            }
            PlaylistKind::Multivariant => {
                self.playlist_parser
                    .items
                    .push(PlaylistItem::MultivariantTag(tag));
                Ok(())
            }

            PlaylistKind::Media => Err(ParseError::MixedPlaylistTypes),
        }
    }

    fn consume_uri(&mut self, uri: String, line_number: usize) -> Result<(), ParseError> {
        let uri: Uri = uri.parse()?;
        match self.playlist_parser.kind {
            PlaylistKind::Unknown => {
                // self.promote_to_media()?;
                // let pseg = self.parse_segment_state.finish_pending_segment();
                // let media_segment = pseg.build(uri.as_str().into())?;
                // self.playlist_parser.items.push(PlaylistItem::Segment(media_segment));

                // self.playlist_parser.items
                //     .push(PlaylistItem::Uri(uri.parse()?, line_number));
            }
            PlaylistKind::Media => {
                let pseg = self.parse_segment_state.finish_pending_segment();
                let media_segment = pseg.build(&uri)?;
                self.playlist_parser
                    .items
                    .push(PlaylistItem::Segment(media_segment));

                // media segment, i think we should store uri's and line number
                // so we could enforce media segment validation - eg tags being for next n occurences
                // of uri until we see that tag again..etc
                self.playlist_parser
                    .items
                    .push(PlaylistItem::Uri(uri, line_number));
            }
            PlaylistKind::Multivariant => {
                // in multivariant playlists, a URI line follows a tag like
                // #EXT-X-STREAM-INF. It points to a sub-playlist.
                // thinking about this atm!!!

                let pstream_inf = self.parse_stream_inf_state.finish();
                let stream_inf = pstream_inf.build(&uri);
                self.playlist_parser
                    .items
                    .push(PlaylistItem::MultivariantTag(
                        MultivariantExclusiveTag::StreamInf(stream_inf),
                    ));

                self.playlist_parser
                    .items
                    .push(PlaylistItem::Uri(uri, line_number));
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Playlist, ParseError> {
        match self.playlist_parser.kind {
            PlaylistKind::Media => {
                let mut media_playlist = MediaPlaylist::default();

                // id prefer to use itertools::zip_longest tbf
                for tag in self.playlist_parser.items {
                    match tag {
                        PlaylistItem::SharedTag(shared) => {
                            media_playlist.apply_shared_tag(shared)?;
                        }
                        PlaylistItem::MediaTag(media) => {
                            media_playlist.apply_exclusive_tag(media)?;
                        }
                        PlaylistItem::Segment(seg) => {
                            media_playlist
                                .items
                                .push(crate::media::MediaPlaylistItem::MediaSegment(seg));
                        }
                        PlaylistItem::Metadata(metadata) => {
                            media_playlist
                                .items
                                .push(crate::media::MediaPlaylistItem::Metadata(metadata));
                        }
                        _ => {}
                    }
                }

                Ok(Playlist::Media(media_playlist))
            }
            PlaylistKind::Multivariant => {
                let mut multivariant_playlist = MultivariantPlaylist::default();

                for tag in self.playlist_parser.items {
                    match tag {
                        PlaylistItem::MultivariantTag(multi_tag) => {
                            multivariant_playlist.apply_exclusive_tag(multi_tag)?;
                        }
                        PlaylistItem::SharedTag(shared) => {
                            multivariant_playlist.apply_shared_tag(shared)?;
                        }
                        _ => {}
                    }
                }

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

impl FromStr for Playlist {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parse_context = ParseContext::new();

        let mut line_number = 0;
        for raw_line in s.lines() {
            // println!("line: {raw_line:?}");

            line_number += 1;

            let line = parse_context.parse_line_kind(raw_line, line_number)?;

            parse_context.consume(line, line_number)?;
        }

        parse_context.finish()
    }
}

impl StructuralValidator {
    fn consume_media_tag(&mut self, tag: &MediaExclusiveTag) -> Result<(), ParseError> {
        match tag {
            MediaExclusiveTag::MediaSequence(_) => {
                if self.seen_first_media_segment {
                    return Err(ParseError::BadOrder {
                        expected: "Media Sequence before first media segment tag",
                        found: "Media segment tag after Media Sequence",
                    });
                }
            }

            MediaExclusiveTag::DiscontinuitySequence(_) => {
                if self.seen_first_media_segment {
                    return Err(ParseError::BadOrder {
                        expected: "Discontinuity Sequence before first media segment tag",
                        found: "Discontinuity media segment tag after first Media segment tag",
                    });
                }

                if self.seen_discontinuity {
                    return Err(ParseError::BadOrder {
                        expected: "Discontinuity Sequence before any Discontinuity media segment tag",
                        found: "Discontinuity media segment tag after Discontinuity Sequence",
                    });
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn consume_segment(&mut self, segment: &MediaSegment) {
        self.seen_first_media_segment = true;

        if segment.get_discontinuity() {
            self.seen_discontinuity = true;
        }
    }
}

type AsMediaFn = fn(&Playlist) -> Option<&MediaPlaylist>;

impl Playlist {
    pub(crate) const EXTM3U: &'static str = "#EXTM3U";

    pub(crate) fn as_media(&self) -> Option<&MediaPlaylist> {
        if let Playlist::Media(media) = self {
            Some(media)
        } else {
            None
        }
    }

    pub(crate) fn as_multivariant(&self) -> Option<&MultivariantPlaylist> {
        if let Playlist::Multivariant(multivariant) = self {
            Some(multivariant)
        } else {
            None
        }
    }
}

impl Display for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", Self::EXTM3U)?;
        match self {
            Playlist::Media(media_playlist) => {
                write!(f, "{media_playlist}")
            }
            Playlist::Multivariant(multivariant_playlist) => {
                write!(f, "{multivariant_playlist}")
            }
        }
    }
}

impl Default for PlaylistParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            parse_stream_inf_state: StreamInfParserState::default(),
            parse_segment_state: ParseSegmentState::default(),
            playlist_parser: PlaylistParser::default(),
        }
    }
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;
    use std::{fs, path::Path};

    use crate::{
        media::PlayListType, multivariant::MultivariantPlaylistItem,
        parser::PlaylistKind::Multivariant, uri::Uri,
    };

    use super::*;

    static ROOT: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn simple_media() {
        let playlist_file = Path::new(ROOT)
            .join("examples")
            .join("01-dead-simple-media.m3u8");

        let playlist = parse_file_into_playlist(playlist_file);

        match playlist {
            Ok(p) => {
                assert!(matches!(p, Playlist::Media(_)));

                let media = p.as_media();
                assert!(media.is_some());

                if let Some(media) = media {
                    println!("{}", media.get_segments().len());
                    assert!(media.get_shared_tags().contains(&&SharedTag::Version(3)));
                    assert!(
                        media
                            .get_exclusive_tags()
                            .contains(&&MediaExclusiveTag::TargetDuration(10))
                    );
                    let segments = media.get_segments();
                    assert_eq!(segments.len(), 3);
                    let mut iter = segments.iter();
                    assert_eq!(*iter.next().unwrap().get_duration(), 9.009);
                    assert_eq!(*iter.next().unwrap().get_duration(), 9.009);
                    assert_eq!(*iter.next().unwrap().get_duration(), 3.003);
                    assert_eq!(
                        media.get_exclusive_tags().last(),
                        Some(&&MediaExclusiveTag::EndList)
                    );
                };
            }
            Err(e) => panic!("Failed due to: {:?}", e),
        }
    }

    #[test]
    fn media_with_tags() {
        let playlist_file = Path::new(ROOT)
            .join("examples")
            .join("02-media-with-tags.m3u8");

        let playlist = parse_file_into_playlist(playlist_file);

        match playlist {
            Ok(p) => {
                assert!(matches!(p, Playlist::Media(_)));

                let media = p.as_media();
                assert!(media.is_some());

                if let Some(media) = media {
                    println!("{}", media.get_segments().len());
                    assert!(media.get_shared_tags().contains(&&SharedTag::Version(7)));
                    assert!(
                        media
                            .get_shared_tags()
                            .contains(&&SharedTag::IndependentSegments)
                    );
                    assert!(media.get_shared_tags().contains(&&SharedTag::Start {
                        time_offset: 0.0,
                        precise: true
                    }));
                    assert!(
                        media
                            .get_exclusive_tags()
                            .contains(&&MediaExclusiveTag::TargetDuration(8))
                    );
                    assert!(
                        media
                            .get_exclusive_tags()
                            .contains(&&MediaExclusiveTag::MediaSequence(42))
                    );
                    assert!(
                        media
                            .get_exclusive_tags()
                            .contains(&&MediaExclusiveTag::PlaylistType(PlayListType::Vod))
                    );
                    assert!(
                        media
                            .get_exclusive_tags()
                            .contains(&&MediaExclusiveTag::EndList)
                    );
                    let segments = media.get_segments();
                    assert_eq!(segments.len(), 3);

                    for (i, seg) in segments.iter().enumerate() {
                        println!("{:?}", seg);
                        assert_eq!(*seg.get_duration(), 8.000);
                        assert_eq!(
                            seg.get_uri(),
                            &<&str as Into<Uri>>::into(
                                format! {"segment-000{}.ts", i + 1}.as_str()
                            )
                        );
                        if i == 0 {
                            assert_eq!(seg.get_title(), Some("Episode intro".into()));
                        } else {
                            assert_eq!(seg.get_title(), Some(format!("Episode segment {}", i + 1)));
                        }
                    }

                    assert_eq!(
                        media.get_exclusive_tags().last(),
                        Some(&&MediaExclusiveTag::EndList)
                    );
                }
            }
            Err(e) => panic!("Failed due to: {:?}", e),
        }
    }

    #[test]
    fn test_simple_back_and_forth() {
        let playlist_file = Path::new(ROOT)
            .join("examples")
            .join("01-dead-simple-media.m3u8");

        let playlist = parse_file_into_playlist(playlist_file.clone()).unwrap();
        let original = std::fs::read_to_string(playlist_file).unwrap();

        let rendered = playlist.to_string();
        println!("rendered: {}", rendered);
        println!("playlist: {:?}", playlist);

        let reparsed = Playlist::from_str(&rendered).unwrap();

        assert_eq!(original, rendered);
        assert_eq!(playlist, reparsed);
    }

    #[test]
    fn test_all_playlists() {
        let examples_dir = Path::new(ROOT).join("examples");

        let entries = fs::read_dir(&examples_dir).expect("Failed to read examples directory");

        for (i, entry) in entries.enumerate() {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "m3u8") {
                println!("Testing file: {:?}", path);

                let playlist = parse_file_into_playlist(&path).unwrap();
                let rendered = playlist.to_string();
                // fs::write(examples_dir.join(format!("rendered-{i}.m3u8")), &rendered).unwrap();
                let reparsed = Playlist::from_str(&rendered).unwrap();

                assert_eq!(playlist, reparsed);
            }
        }
    }

    #[test]
    fn test_multivariant_complex() {
        let playlist_file = Path::new(ROOT)
            .join("examples")
            .join("05-multivariant-complex.m3u8");

        let playlist = parse_file_into_playlist(playlist_file.clone()).unwrap();
        let original = std::fs::read_to_string(playlist_file).unwrap();

        let rendered = playlist.to_string();

        // println!("playlist: {:?}", playlist);

        let reparsed = Playlist::from_str(&rendered).unwrap();
        // assert_eq!(original, rendered);
        assert_eq!(playlist, reparsed);
    }
}
