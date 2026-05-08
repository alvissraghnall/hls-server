use crate::{playlist::SharedTag, types::AttributeList};

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
    Common(SharedTag),
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
