use crate::{playlist::SharedTag, attribute_list::AttributeList};

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
