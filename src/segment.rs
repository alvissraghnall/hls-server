use std::str::FromStr;

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone as _};

use crate::{
    attribute_list::{AttributeList, parse_attribute_list},
    error::ParseError,
    uri::Uri,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MediaSegment {
    uri: Uri,
    byte_range: Option<ByteRange>, // not entirely sure about this just yet
    duration: DurationValue,       // trying not to use floats, but gats to
    title: Option<String>,
    media_sequence: Option<u64>,
    discontinuity: bool,
    key: Option<Key>,
    map: Option<Map>,
    program_date_time: Option<DateTime<FixedOffset>>,
    gap: bool,
    bitrate: Option<u64>,
    part: Option<PartialSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ByteRange {
    len: u64,
    offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DurationValue {
    Int(u32),
    Float(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Map {
    uri: Uri,
    byte_range: Option<ByteRange>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Key {
    method: Method,
    uri: Uri,
    iv: Option<Vec<u8>>,
    key_format: Option<String>,
    key_format_versions: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Method {
    None,
    Aes128,
    SampleAes,
    SampleAesCtr,
    Aes256Gcm,
}

pub(crate) struct ParseSegmentState {
    current_key: Option<Key>,
    current_map: Option<Map>,
    pending_segment: Option<PendingSegment>,
    previous_byterange_end: Option<u64>,
    previous_byterange_uri: Option<String>,
    current_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PartialSegment {
    uri: Uri,
    duration: f64,
    independent: Option<bool>,
    byte_range: Option<ByteRange>,
    gap: bool,
}

pub(crate) struct PendingSegment {
    duration: Option<DurationValue>, // compulsory // should be int for compat v < 3
    title: Option<String>,
    byte_range: Option<ByteRange>,
    discontinuity: bool,
    program_date_time: Option<DateTime<FixedOffset>>,
    gap: bool,
    bitrate: Option<u64>,
    part: Option<PartialSegment>,
    key: Option<Key>, // ????????????????
    map: Option<Map>,
}

impl MediaSegment {
    pub(crate) fn new(
        uri: Uri,
        byte_range: Option<ByteRange>,
        duration: DurationValue,
        title: Option<String>,
        media_sequence: Option<u64>,
        discontinuity: bool,
        key: Option<Key>,
        map: Option<Map>,
        program_date_time: Option<DateTime<FixedOffset>>,
        gap: bool,
        bitrate: Option<u64>,
        part: Option<PartialSegment>,
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
    pub(crate) fn get_duration(&self) -> &DurationValue {
        &self.duration
    }

    pub(crate) fn get_title(&self) -> Option<String> {
        self.title.clone()
    }

    pub(crate) fn get_byte_range(&self) -> Option<&ByteRange> {
        self.byte_range.as_ref()
    }

    pub(crate) fn get_uri(&self) -> &Uri {
        &self.uri
    }

    pub(crate) fn get_media_sequence(&self) -> Option<u64> {
        self.media_sequence
    }

    pub(crate) fn get_key(&self) -> Option<&Key> {
        self.key.as_ref()
    }

    pub(crate) fn get_map(&self) -> Option<&Map> {
        self.map.as_ref()
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
                // let duration = extinf[0]
                //     .parse::<f32>()
                //     .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                let duration = extinf[0].parse()?;
                let title = if extinf.len() == 2 {
                    Some(extinf[1].to_string())
                } else {
                    None
                };
                self.duration = Some(duration);
                self.title = title;
            }
            tag if tag.starts_with("#EXT-X-BYTERANGE:") => {
                let byterange = tag["#EXT-X-BYTERANGE:".len()..]
                    .splitn(2, '@')
                    .collect::<Vec<_>>();
                let len = byterange[0]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                let offset = if byterange.len() == 2 {
                    Some(
                        byterange[1]
                            .parse::<u64>()
                            .map_err(|_| ParseError::InvalidLine(line.to_string()))?,
                    )
                } else {
                    None
                };
                self.byte_range = Some(ByteRange { len, offset });
            }
            tag if tag.starts_with("#EXT-X-DISCONTINUITY") => {
                self.discontinuity = true;
            }
            tag if tag.starts_with("#EXT-X-KEY:") => {
                let attr_str = &tag["#EXT-X-KEY:".len()..];
                let attrs = parse_attribute_list(attr_str)?;
                let key = Key::try_from(attrs)?;
                self.key = Some(key);
            }
            tag if tag.starts_with("#EXT-X-MAP:") => {
                let attr_str = &tag["#EXT-X-MAP:".len()..];
                let attrs = parse_attribute_list(attr_str)?;

                let map = Map::try_from(attrs)?;
                self.map = Some(map);
            }
            tag if tag.starts_with("#EXT-X-PROGRAM-DATE-TIME:") => {
                let datetime_str = &tag["#EXT-X-PROGRAM-DATE-TIME:".len()..];
                let datetime = parse_datetime(datetime_str)
                    .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                self.program_date_time = Some(datetime);
            }
            tag if tag.starts_with("#EXT-X-GAP") => self.gap = true,
            tag if tag.starts_with("#EXT-X-BITRATE:") => {
                let bitrate = tag["#EXT-X-BITRATE:".len()..]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidLine(line.to_string()))?;
                self.bitrate = Some(bitrate)
            }
            tag if tag.starts_with("#EXT-X-PART:") => {
                let attr_str = &tag["#EXT-X-PART:".len()..];
                let attrs = parse_attribute_list(attr_str)?;

                let partial_segment = PartialSegment::try_from(attrs)?;
                self.part = Some(partial_segment);
            }
            _ => return Err(ParseError::InvalidLine(line.to_string())),
        }

        Ok(())
    }

    pub(crate) fn build(self, uri: Uri) -> Result<MediaSegment, ParseError> {
        MediaSegment::try_from(self).map(|mut seg| {
            seg.uri = uri;
            seg
        })
    }
}

impl ParseSegmentState {
    pub(crate) fn new() -> Self {
        Self {
            current_key: None,
            current_map: None,
            pending_segment: Some(PendingSegment::new()),
            previous_byterange_end: None,
            previous_byterange_uri: None,
            current_uri: None,
        }
    }
    pub(crate) fn parse_line(&mut self, line: &str) -> Result<(), ParseError> {
        self.pending_segment
            .as_mut()
            .ok_or(ParseError::NoPendingSegment)?
            .parse(line)
    }

    pub(crate) fn take_pending_segment(&mut self) -> Option<PendingSegment> {
        let pseg = self.pending_segment.take();
        self.pending_segment = Some(PendingSegment::new());
        pseg
    }
}

impl Default for ParseSegmentState {
    fn default() -> Self {
        Self::new()
    }
}

impl Key {
    pub(crate) fn get_method(&self) -> &Method {
        &self.method
    }

    pub(crate) fn get_uri(&self) -> &Uri {
        &self.uri
    }

    pub(crate) fn get_iv(&self) -> Option<&Vec<u8>> {
        self.iv.as_ref()
    }

    pub(crate) fn get_key_format(&self) -> Option<&str> {
        self.key_format.as_deref()
    }

    pub(crate) fn get_key_format_versions(&self) -> &[u16] {
        self.key_format_versions.as_ref()
    }
}

impl TryFrom<AttributeList> for Key {
    type Error = ParseError;

    fn try_from(map: AttributeList) -> Result<Self, Self::Error> {
        let method = map
            .get("METHOD")
            .and_then(|v| v.as_enumerated_string())
            .ok_or_else(|| ParseError::MissingAttribute {
                attribute: "METHOD".into(),
            })?;

        let method_enum = match method {
            "NONE" => Method::None,
            "AES-128" => Method::Aes128,
            "SAMPLE-AES" => Method::SampleAes,
            "SAMPLE-AES-CTR" => Method::SampleAesCtr,
            "AES-256-GCM" => Method::Aes256Gcm,
            _ => {
                return Err(ParseError::InvalidAttributeValue {
                    attribute: "METHOD".into(),
                    value: method.into(),
                    expected: "one of NONE, AES-128, SAMPLE-AES, SAMPLE-AES-CTR, AES-256-GCM"
                        .into(),
                });
            }
        };

        if method_enum == Method::None && map.len() > 1 {
            return Err(ParseError::TooManyAttributes {
                expected: 1,
                found: map.len(),
            });
        }

        let uri = map
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .ok_or_else(|| ParseError::MissingAttribute {
                attribute: "URI".into(),
            })?;

        let iv = map.get("IV").and_then(|v| v.as_hex_sequence());

        let key_format = map
            .get("KEYFORMAT")
            .and_then(|v| v.as_quoted_string().map(str::to_owned));

        let key_format_versions = map
            .get("KEYFORMATVERSIONS")
            .and_then(|v| v.as_quoted_string())
            .map(|s| {
                s.split('/')
                    .filter_map(|part| part.parse::<u16>().ok())
                    .collect::<Vec<u16>>()
            })
            .unwrap_or(vec![1]);

        Ok(Key {
            method: method_enum,
            uri: uri.into(),
            iv: iv.map(|v| v.to_owned()),
            key_format,
            key_format_versions,
        })
    }
}

impl TryFrom<AttributeList> for Map {
    type Error = ParseError;

    fn try_from(map: AttributeList) -> Result<Self, Self::Error> {
        let uri = map
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .ok_or_else(|| ParseError::MissingAttribute {
                attribute: "URI".into(),
            })?;

        let byte_range = if let Some(br) = map.get("BYTERANGE") {
            let br_str = br
                .as_quoted_string()
                .ok_or(ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: "NONE".into(),
                    expected: "a quoted string in the format 'length@offset'".into(),
                })?;
            let parts: Vec<&str> = br_str.split('@').collect();

            if parts.len() != 2 {
                return Err(ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: br_str.into(),
                    expected: "a quoted string in the format 'length@offset'".into(),
                });
            }
            let len = parts[0]
                .parse::<u64>()
                .map_err(|_| ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: parts[1].into(),
                    expected: "a valid decimal integer length value".into(),
                })?;

            let offset =
                parts[1]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidAttributeValue {
                        attribute: "BYTERANGE".into(),
                        value: parts[1].into(),
                        expected: "a valid decimal integer offset value".into(),
                    })?;

            Some(ByteRange {
                len,
                offset: Some(offset),
            }) // offset always has value
        } else {
            None
        };

        Ok(Map {
            uri: uri.into(),
            byte_range,
        })
    }
}

impl TryFrom<AttributeList> for PartialSegment {
    type Error = ParseError;

    fn try_from(value: AttributeList) -> Result<Self, Self::Error> {
        let uri: Uri = value
            .get("URI")
            .and_then(|v| v.as_quoted_string())
            .map(|v| v.into())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "URI".into(),
                value: "NONE".into(),
                expected: "a quoted string that is a valid uri".into(),
            })?;

        let duration = value
            .get("DURATION")
            .and_then(|v| v.as_decimal_floating_point())
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "DURATION".into(),
                value: "NONE".into(),
                expected: "a decimal floating point number".into(),
            })?;

        let independent = value.get("INDEPENDENT").and_then(|v| {
            if v.as_enumerated_string() == Some("YES") {
                Some(true)
            } else {
                None
            }
        });

        let gap = value
            .get("GAP")
            .and_then(|v| v.as_enumerated_string())
            .map(|v| v == "YES")
            .unwrap_or(false);

        let byte_range = if let Some(br) = value.get("BYTERANGE") {
            let parts = br
                .as_quoted_string()
                .ok_or(ParseError::ExpectedQuotedString)?
                .splitn(2, '@')
                .collect::<Vec<_>>();

            let len = parts[0]
                .parse::<u64>()
                .map_err(|_| ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: parts[0].into(),
                    expected: "a valid decimal integer length value".into(),
                })?;

            let offset =
                if parts.len() == 2 {
                    Some(parts[1].parse::<u64>().map_err(|_| {
                        ParseError::InvalidAttributeValue {
                            attribute: "BYTERANGE".into(),
                            value: parts[1].into(),
                            expected: "a valid decimal integer offset value".into(),
                        }
                    })?)
                } else {
                    None
                };

            Some(ByteRange { len, offset })
        } else {
            None
        };

        Ok(PartialSegment {
            byte_range,
            duration,
            gap,
            independent,
            uri,
        })
    }
}

impl FromStr for DurationValue {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('.') || s.contains('e') || s.contains('E') {
            Ok(Self::Float(s.parse()?))
        } else {
            match s.parse::<u32>() {
                Ok(n) => Ok(Self::Int(n)),
                Err(_) => Ok(Self::Float(s.parse()?)),
            }
        }
    }
}

impl ToString for DurationValue {
    fn to_string(&self) -> String {
        match self {
            DurationValue::Int(i) => i.to_string(),
            DurationValue::Float(f) => f.to_string(),
        }
    }
}

impl PartialEq<f32> for DurationValue {
    fn eq(&self, other: &f32) -> bool {
        match self {
            DurationValue::Int(n) => (*n as f32) == *other,
            DurationValue::Float(n) => *n == *other,
        }
    }
}

impl DurationValue {
    pub(crate) fn round(&self) -> i64 {
        match self {
            DurationValue::Int(n) => *n as i64,
            DurationValue::Float(n) => n.round() as i64,
        }
    }
}

impl PartialEq<u32> for DurationValue {
    fn eq(&self, other: &u32) -> bool {
        match self {
            DurationValue::Int(n) => n == other,
            DurationValue::Float(n) => *n == (*other as f32),
        }
    }
}

pub(crate) fn parse_datetime(input: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt);
    }
    if let Ok(dt) = DateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S%.3f%:z") {
        return Ok(dt);
    }

    // if no timezone parse as naive and make UTC (+00:00)
    let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S%.3f")?;
    let utc_offset = FixedOffset::east_opt(0).unwrap();
    Ok(utc_offset.from_utc_datetime(&naive))
}

impl TryFrom<PendingSegment> for MediaSegment {
    type Error = ParseError;

    fn try_from(value: PendingSegment) -> Result<Self, Self::Error> {
        if value.duration.is_none() {
            return Err(ParseError::MissingAttribute {
                attribute: "DURATION".into(),
            });
        }
        Ok(Self {
            uri: "".into(),
            byte_range: value.byte_range,
            duration: value.duration.unwrap(),
            title: value.title,
            media_sequence: None, // for now
            discontinuity: value.discontinuity,
            gap: value.gap,
            key: value.key,
            map: value.map,
            program_date_time: value.program_date_time,
            bitrate: value.bitrate,
            part: value.part,
        })
    }
}
