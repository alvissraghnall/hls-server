use crate::{
    error::ValidationError,
    media::{MediaExclusiveTag, MediaTag},
    parser::Playlist,
    playlist::SharedTag,
    segment::{DurationValue, MediaSegment},
};

impl Playlist {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // wait, can i just call validate() fn of `Tag` trait on both ?
        // same time ? dont think so lol
        //
        match self {
            Playlist::Media(media_playlist) => {
                let version = media_playlist.tags.iter().find_map(|tag| {
                    if let MediaTag::Shared(t) = tag {
                        if let SharedTag::Version(v) = t {
                            return Some(v);
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

                // for (i, tag) in media_playlist.tags.iter().enumerate() {
                //     if let MediaTag::Shared(excl) = tag {
                //         if let SharedTag
                //     }
                // }
                for seg in media_playlist.segments.iter() {
                    if let Some(key) = seg.get_key() {
                        if let Some(iv) = key.get_iv() {
                            if let Some(v) = version {
                                if v < &2u8 {
                                    return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 2 or higher if it contains the IV attribute of the EXT-X-KEY tag.".into()));
                                }
                            } else {
                                return Err(ValidationError::MissingRequiredTag(
                                    "#EXT-X-VERSION".into(),
                                ));
                            }
                        }
                    }

                    if let DurationValue::Float(duration) = seg.get_duration() {
                        if let Some(v) = version {
                            if *v < 3u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 3 or higher if it contains Floating-point EXTINF duration values.".into()));
                            }
                        } else {
                            return Err(ValidationError::MissingRequiredTag(
                                "#EXT-X-VERSION".into(),
                            ));
                        }
                    }

                    if let Some(br) = seg.get_byte_range() {
                        if let Some(v) = version {
                            if *v < 4u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 4 or higher if it contains The EXT-X-BYTERANGE tag.".into()));
                            }
                        } else {
                            return Err(ValidationError::MissingRequiredTag(
                                "#EXT-X-VERSION".into(),
                            ));
                        }
                    }

                    // isnt all that overly repititive ?? UP UP
                    // UPPPPPPPPPPPPPPPPPPPPPP
                    // REMEMBER: A
                    // Playlist that contains tags or attributes that are not compatible
                    // with protocol version 1 MUST include an EXT-X-VERSION tag.
                }
            }
            Playlist::Multivariant(multivariant_playlist) => todo!(),
        };

        unimplemented!()
    }
}
