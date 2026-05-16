use chrono::{DateTime, Utc};

use crate::{attribute_list::{AttributeList, parse_attribute_list}, error::ParseError, uri::Uri};

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
    offset: Option<u64>,
}

pub(crate) struct Map {
    uri: String,
    byte_range: ByteRange,
}

pub(crate) struct Key {
    method: Method,
    uri: Uri,
    iv: Option<u128>,
    key_format: Option<String>,
    key_format_versions: Option<Vec<u16>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Method {
    None,
    Aes128,
    SampleAes,
    SampleAesCtr,
    Aes256Gcm,
}

struct ParserState {
    current_key: Option<Key>,
    current_map: Option<Map>,
    pending_segment: PendingSegment,
    previous_byterange_end: Option<u64>,
    previous_byterange_uri: Option<String>,
    current_uri: Option<String>,
}

struct PendingSegment {
    duration: Option<f32>, // compulsory // should be int for compat v < 3
    title: Option<String>,
    byte_range: Option<ByteRange>,
    discontinuity: bool,
    program_date_time: Option<DateTime<Utc>>,
    gap: bool,
    bitrate: Option<u64>,
    part: Option<AttributeList>,
    key: Option<Key>, // ????????????????
    map: Option<Map>,
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

impl PendingSegment {
    fn new() -> Self {
        Self {
            duration: None,
            title: None,
            byte_range: None,
            discontinuity: false,
            program_date_time: None,
            gap: false,
            bitrate: None,
            part: None,
            key: None,
            map: None,
        }
    }

    fn reset_segment_scoped_fields(&mut self) {
        // reset fields that are scoped to the segment
        self.duration = None;
        self.title = None;
        self.byte_range = None;
    }

    // should return an actual MediaSegment, ensure to change soon
    // it'd error out anyways haha
    fn parse(&mut self, line: &str) -> Result<(), ParseError> {
        match line {
            tag if tag.starts_with("#EXTINF:") => {
                let extinf = tag["#EXTINF:".len()..].splitn(2, ',').collect::<Vec<_>>();
                let duration = extinf[0]
                    .parse::<f32>()
                    .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                let title = if extinf.len() == 2 {
                    Some(extinf[1].to_string())
                } else {
                    None
                };
                self.duration = Some(duration);
                self.title = title;
            }
            tag if tag.starts_with("#EXT-X-BYTERANGE") => {
                let byterange = tag["#EXT-X-BYTERANGE:".len()..].splitn(2, '@').collect::<Vec<_>>();
                let len = byterange[0]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                let offset = if byterange.len() == 2 {
                    Some(byterange[1]
                        .parse::<u64>()
                        .map_err(|_| ParseError::InvalidLine(line.to_string()))?)
                } else {
                    None
                };
                self.byte_range = Some(ByteRange { len, offset });
            }
            tag if tag.starts_with("#EXT-X-DISCONTINUITY") => {
                self.discontinuity = true;
            }
            tag if tag.starts_with("#EXT-X-KEY") => {
                let attr_str = &tag["#EXT-X-KEY:".len()..];
                let attrs = parse_attribute_list(attr_str)?;
                let key = Key::try_from(attrs)?;
                self.key = Some(key);
            }
            _ => return Err(ParseError::InvalidLine(line.to_string())),
        }

        Ok(())
    }
}

impl ParserState {
    fn new() -> Self {
        Self {
            current_key: None,
            current_map: None,
            pending_segment: PendingSegment::new(),
            previous_byterange_end: None,
            previous_byterange_uri: None,
            current_uri: None,
        }
    }
}

impl Default for ParserState {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<AttributeList> for Key {
    type Error = ParseError;

    fn try_from(value: AttributeList) -> Result<Self, Self::Error> {
        let method = value
            .get("METHOD")
            .and_then(|v| v.as_enumerated_string())
            .ok_or_else(|| ParseError::NoAttribute)?;

        let method_enum = match method {
            "NONE" => Method::None,
            "AES-128" => Method::Aes128,
            "SAMPLE-AES" => Method::SampleAes,
            "SAMPLE-AES-CTR" => Method::SampleAesCtr,
            "AES-256-GCM" => Method::Aes256Gcm,
            _ => return Err(ParseError::InvalidAttributeValue(method.to_string())),
        };

        if method_enum == Method::None && value.len() > 1 {
            return Err(ParseError::TooManyAttributes);
        }

        let uri = value
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .ok_or_else(|| ParseError::NoAttribute)?;

        let iv = value
            .get("IV")
            .and_then(|v| v.as_hex_sequence());

        let key_format = value
            .get("KEYFORMAT")
            .and_then(|v| v.as_quoted_string().map(str::to_owned));

        let key_format_versions = value
            .get("KEYFORMATVERSIONS")
            .and_then(|v| v.as_quoted_string())
            .map(|s| {
                s.split('/')
                    .filter_map(|part| part.parse::<u16>().ok())
                    .collect::<Vec<u16>>()
            });

        Ok(Key {
            method: method_enum,
            uri: uri.into(),
            iv,
            key_format,
            key_format_versions,
        })
    }
}