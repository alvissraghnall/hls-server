use crate::{
    error::ValidationError,
    media::{MediaExclusiveTag, MediaTag},
    multivariant::{InStreamId::{self, Service}, Media, MultivariantExclusiveTag, MultivariantTag},
    parser::Playlist,
    playlist::SharedTag,
    segment::{DurationValue, Method},
    shared::Tag,
};

macro_rules! require_version {
    ($version:expr, $cond:expr, $min:expr, $msg:expr) => {
        if $cond && *$version < $min {
            return Err(ValidationError::InvalidSharedTag(
                SharedTag::Version(*$version),
                $msg.into(),
            ));
        }
    };
}

impl Tag for Playlist {
    fn validate(&self) -> Result<(), ValidationError> {
        let version = self
            .version()
            .ok_or_else(|| ValidationError::MissingRequiredTag("#EXT-X-VERSION".into()))?;

        match self {
            //  Note that in protocol version 6, the semantics of the EXT-
            // X-TARGETDURATION tag changed slightly.  In protocol version 5 and
            // earlier it indicated the maximum segment duration; in protocol
            // version 6 and later it indicates the maximum segment duration rounded
            // to the nearest integer number of seconds.
            Playlist::Media(media_playlist) => {
                let has_iframes_only = media_playlist
                    .tags
                    .contains(&MediaTag::Exclusive(MediaExclusiveTag::IFramesOnly));

                for seg in &media_playlist.segments {
                    if let Some(key) = seg.get_key() {
                        require_version!(
                            version,
                            key.get_iv().is_some(),
                            2,
                            "A Media Playlist MUST indicate an EXT-X-VERSION of 2 or higher if it \
                             contains the IV attribute of the EXT-X-KEY tag."
                        );

                        require_version!(
                            version,
                            key.get_method() == &Method::SampleAes,
                            5,
                            "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it \
                             contains the EXT-X-KEY tag with the METHOD attribute set to SAMPLE-AES."
                        );

                        require_version!(
                            version,
                            key.get_key_format().is_some()
                                || !key.get_key_format_versions().is_empty(),
                            5,
                            "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it \
                             contains the KEYFORMAT and KEYFORMATVERSIONS attributes of the EXT-X-KEY tag."
                        );
                    }

                    require_version!(
                        version,
                        matches!(seg.get_duration(), DurationValue::Float(_)),
                        3,
                        "A Media Playlist MUST indicate an EXT-X-VERSION of 3 or higher if it \
                         contains Floating-point EXTINF duration values."
                    );

                    require_version!(
                        version,
                        seg.get_byte_range().is_some(),
                        4,
                        "A Media Playlist MUST indicate an EXT-X-VERSION of 4 or higher if it \
                         contains The EXT-X-BYTERANGE tag."
                    );

                    require_version!(
                        version,
                        seg.get_map().is_some(),
                        5,
                        "A Media Playlist MUST indicate an EXT-X-VERSION of 5 or higher if it \
                         contains the EXT-X-MAP tag."
                    );
                }

                require_version!(
                    version,
                    has_iframes_only,
                    4,
                    "A Media Playlist MUST indicate an EXT-X-VERSION of 4 or higher if it \
                     contains the EXT-X-I-FRAMES-ONLY tag."
                );

                if !has_iframes_only {
                    require_version!(
                        version,
                        media_playlist
                            .segments
                            .iter()
                            .any(|s| s.get_map().is_some()),
                        6,
                        "A Media Playlist MUST indicate an EXT-X-VERSION of 6 or higher if it \
                         contains the EXT-X-MAP tag in a Media Playlist that does not contain EXT-X-I-FRAMES-ONLY."
                    );
                }
            }

            Playlist::Multivariant(multivariant_playlist) => {
                multivariant_playlist.exclusive_tags.iter().try_for_each(|t| match t {
                    MultivariantExclusiveTag::Media(media) => {
                        require_version!(
                            version,
                            media.get_instream_id().map(|iid| iid.is_service()).unwrap_or(false),
                            7,
                            "A Multivariant Playlist MUST indicate an EXT-X-VERSION of 7 or higher if it \
                             contains a 'SERVICE' values for the INSTREAM-ID attribute of the EXT-X-MEDIA \
                             tag."
                        );

                        Ok(())
                    }
                    _ => Ok(()),
                })?;
                // .map_err(|e| ValidationError::MissingRequiredTag(e.to_string()))?;

                multivariant_playlist.shared_tags.iter().try_for_each(|t| match t {
                    SharedTag::Variable(_) => {
                        require_version!(
                            version,
                            true,
                            8,
                            "A Multivariant Playlist MUST indicate an EXT-X-VERSION of 8 or higher if it \
                             contains Variable substitution."
                        );

                        Ok(())
                    }
                    
                    _ => Ok(()),
                })?;
                
            }
        }

        unimplemented!()
    }
}

impl Playlist {
    fn version(&self) -> Option<&u8> {
        match self {
            Playlist::Media(p) => p.tags.iter().find_map(|tag| match tag {
                MediaTag::Shared(SharedTag::Version(v)) => Some(v),
                _ => None,
            }),
            Playlist::Multivariant(p) => p.shared_tags.iter().find_map(|tag| match tag {
                SharedTag::Version(v) => Some(v),
                _ => None,
            }),
        }
    }
}