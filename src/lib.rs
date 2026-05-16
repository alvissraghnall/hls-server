use std::sync::mpsc::SendError;

use crate::{
    error::ParseError,
    media::{MediaExclusiveTag, parse_media_exclusive_tag},
    shared::parse_shared_tag,
};

mod attribute_list;
mod error;
mod media;
mod multivariant;
mod playlist;
mod segment;
mod shared;
mod uri;
mod key;

/**
 *
 * WORKFLOW:::::
 * say, we want to parse a multivariant playlist
 * 1. reaad the file or sumn
 * 2. ensure it has #EXTM3U as first line, discard if it doesn't
 * 3. loop through the lines:
 * 3.1 if line starts with #, parse it as a tag
 * 3.1.1 say, tag is #EXT-X-VERSION for instance:
 * 3.1.1.1 parse the version number as SharedTag
 * 3.1.1.2 append to MultivariantPlaylist
 * 3.1.1.3 if we encounter same tag again, throw an error
 * 3.1.1.4 but parse_shared_tag only accepts line as input
 * 3.1.1.5 in essence, it can;t tell if it has seen the tag before
 * 3.1.1.6 what to do then?
 *
 * 3.2 if line doesn't start with #, parse it as a media entry
 * 3.3 if line is empty, skip it
 * 4.
 */

fn parse_media_playlist(lines: &[String]) -> Result<media::MediaPlaylist, error::ParseError> {
    let mut playlist = media::MediaPlaylist::default();
    let seen_first_segment = false;
    for line in lines {
        match parse_media_exclusive_tag(line) {
            Ok(tag) => {
                if matches!(
                    tag,
                    MediaExclusiveTag::MediaSequence(_)
                        | MediaExclusiveTag::DiscontinuitySequence(_)
                ) && seen_first_segment
                {
                    return Err(ParseError::MediaSequenceAfterSegment);
                }

                // also ensure X-DISCONTINUITY-SEQUENCE appears
                // before EXT-X-DISCONTINUITY tag
            }

            // and if we encounter a segment URI, set seen_first_segment to true
            _ => {}
        }
    }
    Ok(playlist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = add(2, 2);
        // assert_eq!(result, 4);
    }
}
