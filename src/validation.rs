use crate::{
    error::ValidationError,
    media::{MediaExclusiveTag, MediaTag},
    parser::Playlist,
    playlist::SharedTag,
    segment::{DurationValue, Method}, shared::Tag,
};

impl Tag for Playlist {
    fn validate(&self) -> Result<(), ValidationError> {
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

                if let Some(v) = version {
                    // for (i, tag) in media_playlist.tags.iter().enumerate() {
                    //     if let MediaTag::Shared(excl) = tag {
                    //         if let SharedTag
                    //     }
                    // }
                    for seg in media_playlist.segments.iter() {
                        if let Some(key) = seg.get_key() {
                            if let Some(iv) = key.get_iv() {
                                if *v < 2u8 {
                                    return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 2 or higher if it contains the IV attribute of the EXT-X-KEY tag.".into()));
                                }
                            }
                        }

                        if let DurationValue::Float(duration) = seg.get_duration() {
                            if *v < 3u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 3 or higher if it contains Floating-point EXTINF duration values.".into()));
                            }
                        }

                        if let Some(br) = seg.get_byte_range() {
                            if *v < 4u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 4 or higher if it contains The EXT-X-BYTERANGE tag.".into()));
                            }
                        }

                        if let Some(key) = seg.get_key() {
                            if key.get_method() == &Method::SampleAes {
                                if *v < 5u8 {
                                    return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it contains the EXT-X-KEY tag with the METHOD attribute set to SAMPLE-AES.".into()));
                                }
                            }

                            if key.get_key_format().is_some() || !key.get_key_format_versions().is_empty() {
                                if *v < 5u8 {
                                    return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it contains the KEYFORMAT and KEYFORMATVERSIONS attributes of the EXT-X-KEY tag.".into()));
                                }
                            }
                        }

                        if let Some(map) = seg.get_map() {
                            if *v < 5u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it contains the EXT-X-MAP tag.".into()));
                            }

                            
                        }

                        // isnt all that overly repititive ?? UP UP
                        // UPPPPPPPPPPPPPPPPPPPPPP
                        // REMEMBER: A
                        // Playlist that contains tags or attributes that are not compatible
                        // with protocol version 1 MUST include an EXT-X-VERSION tag.
                    }

                    if media_playlist
                        .tags
                        .contains(&MediaTag::Exclusive(MediaExclusiveTag::IFramesOnly))
                    {
                        if *v < 4u8 {
                            return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 4 or higher if it contains the EXT-X-I-FRAMES-ONLY tag.".into()));
                        }
                    } else {
                        if media_playlist.segments.iter().any(|s| s.get_map().is_some()) {
                            if *v < 6u8 {
                                return Err(ValidationError::InvalidSharedTag(SharedTag::Version(*v), "A Media Playlist MUST indicate an EXT-X-VERSION of 6 or higher if it contains the EXT-X-MAP tag in a Media Playlist that does not contain EXT-X-I-FRAMES-ONLY.".into()));
                            }
                        }
                    }

                } else {
                    return Err(ValidationError::MissingRequiredTag("#EXT-X-VERSION".into()));
                }
            }
            Playlist::Multivariant(multivariant_playlist) => todo!(),
        };

        unimplemented!()
    }
}
