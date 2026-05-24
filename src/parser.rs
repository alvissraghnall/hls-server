use crate::{
    error::{self, ParseError},
    media::{MediaExclusiveTag, MediaPlaylist, MediaTag, parse_media_exclusive_tag},
    multivariant::{MultivariantExclusiveTag, MultivariantPlaylist},
    playlist::SharedTag,
    read_write,
    shared::parse_shared_tag,
};

enum ParsedLine {
    Empty,
    Comment,
    Uri(String),

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
}

enum PlaylistKind {
    Unknown,
    Media,
    Multivariant,
}

pub fn parse_file_into_playlist(
    path: impl AsRef<std::path::Path>,
) -> Result<(), error::PlaylistReadError> {
    let content = read_write::read_from_file(path)?;

    let mut parser = PlaylistParser::new();

    let mut line_number = 0;
    for raw_line in content.lines() {
        line_number += 1;

        let line = parse_line(raw_line)?;

        parser.consume(line, line_number)?;
    }

    parser.finish();

    Ok(())
}

fn parse_line(line: &str) -> Result<ParsedLine, ParseError> {
    let line = line.trim();

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

    Err(ParseError::UnknownTag {
        tag: line.into(),
        span: crate::error::Span { line: 0, column: 0 }, // change soon x
    })
}

impl PlaylistParser {
    pub(crate) fn new() -> Self {
        Self {
            kind: PlaylistKind::Unknown,
            shared_tags: Vec::new(),
            media_tags: Vec::new(),
            multivariant_tags: Vec::new(),
            uris: Vec::new(),
        }
    }
}

impl PlaylistParser {
    fn consume(&mut self, line: ParsedLine, line_number: usize) -> Result<(), ParseError> {
        match line {
            ParsedLine::Empty | ParsedLine::Comment => {}

            ParsedLine::Uri(uri) => {
                self.consume_uri(uri, line_number)?;
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

    fn consume_uri(&mut self, uri: String, line_number: usize) -> Result<(), ParseError> {
        match self.kind {
            PlaylistKind::Unknown => {
                // A standalone URI lowk implies we're in a media playlist
                self.promote_to_media()?;
                self.uris.push((uri, line_number));
            }
            PlaylistKind::Media => {
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

    fn finish(self) -> Playlist {
        match self.kind {
            PlaylistKind::Media => {
                // combine self.media_tags and self.uris into MediaPlaylist
                Playlist::Media(MediaPlaylist::default())
            }
            PlaylistKind::Multivariant => {
                // combine self.multivariant_tags and self.uris into MultivariantPlaylist
                Playlist::Multivariant(MultivariantPlaylist::default())
            }
            PlaylistKind::Unknown => {
                // handle empty/invalid playlists
                // should pro'lly default to Media for now
                Playlist::Media(MediaPlaylist::default())
            }
        }
    }
}
