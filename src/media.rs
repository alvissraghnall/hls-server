use crate::{attribute_list::AttributeList, error::ParseError, playlist::SharedTag};

enum MediaExclusiveTag {
    TargetDuration(u64),
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
}

impl Default for MediaPlaylist {
    fn default() -> Self {
        Self { tags: Vec::new() }
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

            SharedTag::Variables(v) => {
                v.iter().for_each(|var| {});
            }

            SharedTag::Start {
                precise,
                time_offset,
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
}
