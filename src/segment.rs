use chrono::{DateTime, Utc};

use crate::attribute_list::AttributeList;

pub(crate) struct MediaSegment {
    uri: String,
    byte_range: Option<ByteRange>, // not entirely sure about this just yet
    duration: f32,                 // trying not to use floats, but gats to
    title: Option<String>,
    media_sequence: Option<u64>,
    discontinuity: bool,
    key: Option<Key>,
    map: Option<Map>,
    program_date_time: Option<DateTime<Utc>>,
    gap: bool,
    bitrate: Option<u64>,
    part: Option<AttributeList>,
}

pub(crate) struct ByteRange {
    len: u64,
    offset: u64,
}

pub(crate) struct Map {
    uri: String,
    byte_range: ByteRange,
}

pub(crate) struct Key {
    method: Method,
    uri: String,
    iv: Option<u128>,
    key_format: Option<String>,
    key_format_versions: Option<Vec<u16>>,
}

pub(crate) enum Method {
    None,
    Aes128,
    SampleAes,
    SampleAesCtr,
    Aes256Gcm,
}

impl MediaSegment {
    pub(crate) fn new(
        uri: String,
        byte_range: Option<ByteRange>,
        duration: f32,
        title: Option<String>,
        media_sequence: Option<u64>,
        discontinuity: bool,
        key: Option<Key>,
        map: Option<Map>,
        program_date_time: Option<DateTime<Utc>>,
        gap: bool,
        bitrate: Option<u64>,
        part: Option<AttributeList>,
    ) -> Self {
        Self {
            uri,
            byte_range,
            duration,
            title,
            media_sequence,
            discontinuity,
            key,
            map,
            program_date_time,
            gap,
            bitrate,
            part,
        }
    }

    #[inline(always)]
    pub(crate) fn get_duration(&self) -> f32 {
        self.duration
    }
}
