use std::{collections::HashSet, hash::Hash};

use crate::{
    error::ValidationError,
    media::MediaExclusiveTag,
    multivariant::MultivariantExclusiveTag::{self},
    parser::Playlist,
    playlist::{PlayListVariableDefinition, SharedTag},
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

        // A Playlist MUST indicate an EXT-X-VERSION of 11 or higher if it
        // contains:

        // version of playlist with an EXT-X-DEFINE tag with a QUERYPARAM attribute must be 11 or higher.
        require_version!(
            version,
            self.get_shared_tags().iter().any(|t| matches!(t, SharedTag::Variable(PlayListVariableDefinition::QueryParam { name: _, value: _ }))),
            11,
            "A Playlist MUST indicate an EXT-X-VERSION of 11 or higher if it \
               contains an EXT-X-DEFINE tag with QUERYPARAM attribute"
        );

        match self {
            //  Note that in protocol version 6, the semantics of the EXT-
            // X-TARGETDURATION tag changed slightly.  In protocol version 5 and
            // earlier it indicated the maximum segment duration; in protocol
            // version 6 and later it indicates the maximum segment duration rounded
            // to the nearest integer number of seconds.
            Playlist::Media(media_playlist) => {
                let _exclusive_tags = &media_playlist.get_exclusive_tags();

                let has_iframes_only = media_playlist
                    .get_exclusive_tags()
                    .contains(&&MediaExclusiveTag::IFramesOnly);

                let target_duration = media_playlist
                    .get_exclusive_tags()
                    .iter()
                    .find_map(|x| {
                        if let MediaExclusiveTag::TargetDuration(d) = x {
                            return Some(*d);
                        }
                        None
                    })
                    .ok_or_else(|| ValidationError::MissingRequiredTag("#EXT-X-TARGETDURATION".into()))?;
                
                for seg in &media_playlist.get_segments() {
                    let seg_duration = seg.get_duration();

                    // ensure target duration is not exceeded by any segment
                    if seg_duration > &target_duration {
                        return Err(ValidationError::InvalidTargetDuration);
                    }
                    
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
                            .get_segments()
                            .iter()
                            .any(|s| s.get_map().is_some()),
                        6,
                        "A Media Playlist MUST indicate an EXT-X-VERSION of 6 or higher if it \
                         contains the EXT-X-MAP tag in a Media Playlist that does not contain EXT-X-I-FRAMES-ONLY."
                    );
                }

                let mut skip_seen = HashSet::new();

                if let Some(skip) = media_playlist
                    .get_metadata()
                    .iter()
                    .find_map(|m| m.as_skip())
                {
                    if !skip_seen.insert(&skip) {
                        return Err(ValidationError::InvalidMediaMetadata(
                            "A Media Playlist MUST NOT contain more than one EXT-X-SKIP tag.".into(),
                        ));
                    }

                    require_version!(
                        version,
                        true,
                        9,
                        "A Playlist MUST indicate an EXT-X-VERSION of 9 or higher if it \
                         contains the EXT-X-SKIP tag."
                    );

                    require_version!(
                        version,
                        !skip.recently_removed_dateranges.is_empty(),
                        10,
                        "A Playlist MUST indicate an EXT-X-VERSION of 10 or higher if it \
                         contains an EXT-X-SKIP tag that replaces EXT-X-DATERANGE tags in a Playlist Delta Update."
                    );
                }
            }

            Playlist::Multivariant(multivariant_playlist) => {
                multivariant_playlist.get_exclusive_tags().iter().try_for_each(|t| match t {
                    MultivariantExclusiveTag::Media(media) => {
                        require_version!(
                            version,
                            media.get_instream_id().is_some_and(super::multivariant::InStreamId::is_service),
                            7,
                            "A Multivariant Playlist MUST indicate an EXT-X-VERSION of 7 or higher if it \
                             contains a 'SERVICE' values for the INSTREAM-ID attribute of the EXT-X-MEDIA \
                             tag."
                        );

                        require_version!(
                            version,
                            media.get_instream_id().is_some_and(|iid| !iid.is_cc()),
                            13,
                            "A Multivariant Playlist MUST indicate an EXT-X-VERSION of 13 or higher if it \
                             contains an EXT-X-MEDIA tag with INSTREAM-ID attribute for non CLOSED-CAPTIONS \
                             TYPE."
                        );

                        Ok(())
                    }
                    // should acc be until an attribute whose name starts with "REQ-".
                    MultivariantExclusiveTag::IFrameStreamInf(i) => {
                        require_version!(
                            version,
                            i.get_req_video_layout().is_some(),
                            12,
                            "A Multivariant Playlist MUST indicate an EXT-X-VERSION of 12 or higher if it \
                             contains a 'REQUIRED-VIDEO-LAYOUT' attribute of the EXT-X-IFRAME-STREAM-INF \
                             tag."
                        );
                        Ok(())
                    }
                    _ => Ok(()),
                })?;
                // .map_err(|e| ValidationError::MissingRequiredTag(e.to_string()))?;

                multivariant_playlist.get_shared_tags().iter().try_for_each(|t| match t {
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
            Playlist::Media(p) => p.get_shared_tags().iter().find_map(|tag| match tag {
                SharedTag::Version(v) => Some(v),
                _ => None,
            }),
            Playlist::Multivariant(p) => p.get_shared_tags().iter().find_map(|tag| match tag {
                SharedTag::Version(v) => Some(v),
                _ => None,
            }),
        }
    }

    fn get_shared_tags(&self) -> Vec<&SharedTag> {
        match self {
            Playlist::Media(p) => p.get_shared_tags(),
            Playlist::Multivariant(p) => p.get_shared_tags(),
        }
    }
}
