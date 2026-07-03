use std::{fmt::Display, fs, str::FromStr};

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone as _};

use crate::{
    attribute_list::{AttributeList, parse_attribute_list}, error::ParseError, key, shared::Tag, uri::Uri,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MediaSegment {
    uri: Uri,
    byte_range: Option<ByteRange>, // not entirely sure about this just yet
    duration: DurationValue,       // trying not to use floats, but gats to
    title: Option<String>,
    media_sequence: u64,
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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    pending_segment: PendingSegment,
    previous_byterange_end: Option<u64>,
    previous_byterange_uri: Option<String>,
    current_uri: Option<String>,
    media_sequence: u64,
    previous_media_sequence_number: Option<u64>,
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
    media_sequence: u64,
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
            media_sequence: media_sequence.unwrap_or(0),
            discontinuity,
            key,
            map,
            program_date_time,
            gap,
            bitrate,
            part,
        }
    }

    pub(crate) fn get_discontinuity(&self) -> bool {
        self.discontinuity
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

    pub(crate) fn get_part(&self) -> Option<&PartialSegment> {
        self.part.as_ref()
    }

    pub(crate) fn get_media_sequence(&self) -> u64 {
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
            media_sequence: 0,
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
                self.bitrate = Some(bitrate);
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
            pending_segment: PendingSegment::new(),
            previous_byterange_end: None,
            previous_byterange_uri: None,
            current_uri: None,
            previous_media_sequence_number: None,
            media_sequence: 0,
        }
    }

    pub(crate) fn accept(&mut self, line: &str) -> Result<(), ParseError> {
        self.pending_segment.media_sequence = self.media_sequence;
        self.pending_segment.parse(line)
    }

    pub(crate) fn set_media_sequence(&mut self, media_sequence: u64) {
        self.media_sequence = media_sequence;
    }

    pub(crate) fn finish_pending_segment(&mut self) -> PendingSegment {
        self.previous_media_sequence_number = Some(self.media_sequence);
        self.media_sequence += 1;
        // self.pending_segment.reset_segment_scoped_fields();
        std::mem::replace(&mut self.pending_segment, PendingSegment::new())
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
                    expected: "one of NONE, AES-128, SAMPLE-AES, SAMPLE-AES-CTR, AES-256-GCM",
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
            iv: iv.map(std::borrow::ToOwned::to_owned),
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
                    expected: "a quoted string in the format 'length@offset'",
                })?;
            let parts: Vec<&str> = br_str.split('@').collect();

            if parts.len() != 2 {
                return Err(ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: br_str.into(),
                    expected: "a quoted string in the format 'length@offset'",
                });
            }
            let len = parts[0]
                .parse::<u64>()
                .map_err(|_| ParseError::InvalidAttributeValue {
                    attribute: "BYTERANGE".into(),
                    value: parts[1].into(),
                    expected: "a valid decimal integer length value",
                })?;

            let offset =
                parts[1]
                    .parse::<u64>()
                    .map_err(|_| ParseError::InvalidAttributeValue {
                        attribute: "BYTERANGE".into(),
                        value: parts[1].into(),
                        expected: "a valid decimal integer offset value",
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
            .map(std::convert::Into::into)
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "URI".into(),
                value: "NONE".into(),
                expected: "a quoted string that is a valid uri",
            })?;

        let duration = value
            .get("DURATION")
            .and_then(super::attribute_list::AttributeValue::as_decimal_floating_point)
            .ok_or(ParseError::InvalidAttributeValue {
                attribute: "DURATION".into(),
                value: "NONE".into(),
                expected: "a decimal floating point number",
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
            .is_some_and(|v| v == "YES");

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
                    expected: "a valid decimal integer length value",
                })?;

            let offset =
                if parts.len() == 2 {
                    Some(parts[1].parse::<u64>().map_err(|_| {
                        ParseError::InvalidAttributeValue {
                            attribute: "BYTERANGE".into(),
                            value: parts[1].into(),
                            expected: "a valid decimal integer offset value",
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
            uri,
            duration,
            independent,
            byte_range,
            gap,
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

impl PartialEq<f32> for DurationValue {
    fn eq(&self, other: &f32) -> bool {
        match self {
            DurationValue::Int(n) => (*n as f32) == *other,
            DurationValue::Float(n) => *n == *other,
        }
    }
}

impl PartialEq<u64> for DurationValue {
    fn eq(&self, other: &u64) -> bool {
        self.round_u64() == *other
    }
}

impl PartialOrd<u64> for DurationValue {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.round_u64().partial_cmp(other)
    }
}

impl DurationValue {
    pub(crate) fn round(&self) -> i64 {
        match self {
            DurationValue::Int(n) => i64::from(*n),
            DurationValue::Float(n) => n.round() as i64,
        }
    }

    pub(crate) fn round_u64(&self) -> u64 {
        match self {
            DurationValue::Int(n) => u64::from(*n),
            DurationValue::Float(n) => n.round() as u64,
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
            media_sequence: value.media_sequence,
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

impl Display for DurationValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DurationValue::Int(i) => write!(f, "#EXTINF:{i}"),
            DurationValue::Float(fl) => write!(f, "#EXTINF:{fl}"),
        }
    }
}

impl Display for ByteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // #EXT-X-BYTERANGE:<n>[@<o>]
        write!(f, "#EXT-X-BYTERANGE:{}", self.len)?;
        if let Some(offset) = self.offset {
            write!(f, "@{offset}")?;
        }
        Ok(())
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Aes128 => write!(f, "AES-128"),
            Method::None => write!(f, "NONE"),
            Method::SampleAes => write!(f, "SAMPLE-AES"),
            Method::SampleAesCtr => write!(f, "SAMPLE-AES-CTR"),
            Method::Aes256Gcm => write!(f, "AES-256-GCM"),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#EXT-X-KEY:METHOD={}", self.method)?;
        write!(f, ",URI=\"{}\"", self.uri)?;
        if let Some(iv) = &self.iv {
            write!(f, ",IV=0x{}", hex::encode(iv))?;
        }
        if let Some(key_format) = &self.key_format {
            write!(f, ",KEYFORMAT=\"{key_format}\"")?;
        }
        for (i, version) in self.key_format_versions.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{version}")?;
        }

        write!(f, "\"")?;

        Ok(())
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#EXT-X-MAP:URI=\"{}\"", self.uri)?;
        if let Some(byte_range) = &self.byte_range {
            write!(f, ",BYTERANGE=\"{byte_range}\"")?;
        }

        Ok(())
    }
}

impl Display for PartialSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#EXT-X-PART:URI=\"{}\"", self.uri)?;
        write!(f, ",DURATION={}", self.duration)?;
        if let Some(independent) = self.independent {
            if independent {
                write!(f, ",INDEPENDENT=YES")?;
            } else {
                write!(f, ",INDEPENDENT=NO")?;
            }
        }
        if let Some(byte_range) = &self.byte_range {
            write!(f, ",BYTERANGE=\"{byte_range}\"")?;
        }
        if self.gap {
            write!(f, ",GAP=\"YES\"")?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Display for MediaSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let MediaSegment {
            uri,
            duration,
            title,
            byte_range,
            discontinuity,
            key,
            map,
            program_date_time,
            bitrate,
            gap,
            part,
            media_sequence: _,
        } = self;

        write!(f, "{duration}")?;
        if let Some(title) = title {
            write!(f, ",{title}")?;
        }
        writeln!(f)?;

        if let Some(byte_range) = byte_range {
            writeln!(f, "{byte_range}")?;
        }

        if *discontinuity {
            writeln!(f, "#EXT-X-DISCONTINUITY")?;
        }

        if let Some(key) = key {
            writeln!(f, "{key}")?;
        }

        if let Some(map) = map {
            writeln!(f, "{map}")?;
        }
        if let Some(program_date_time) = program_date_time {
            writeln!(f, "#EXT-X-PROGRAM-DATE-TIME:{program_date_time}")?;
        }
        if *gap {
            writeln!(f, "#EXT-X-GAP")?;
        }
        if let Some(bitrate) = bitrate {
            writeln!(f, "#EXT-X-BITRATE:{bitrate}")?;
        }
        if let Some(part) = part {
            writeln!(f, "{part}")?;
        }

        writeln!(f, "{uri}")?;

        Ok(())
    }
}

impl MediaSegment {
    pub fn decrypt(&self) -> Result<Vec<u8>, String> {
        if let Some(key) = &self.key {
            let key_bytes = {
                if key.method != Method::Aes128 && key.method != Method::Aes256Gcm {
                    return Err("Unsupported encryption method".to_string());
                }
                let key_data = fs::read(&key.uri.to_string()).map_err(|e| e.to_string())?;
                key_data
            };

            let iv = if key.method == Method::Aes128 {
                key.iv.clone().unwrap_or_else(|| {
                    let seq_num = self.media_sequence;
                    seq_num.to_be_bytes().to_vec()
                })
            } else {
                Vec::with_capacity(0)
            };

            // if key.method == Method::Aes128 {
            //     if key_bytes.len() != 16 {
            //         return Err("Invalid AES-128 key length".to_string());
            //     }
            //     crate::key::decrypt::decrypt_aes_128(&key_bytes, &iv, &self.uri.to_string())
            // } else if key.method == Method::Aes256Gcm {
            //     if key_bytes.len() != 32 {
            //         return Err("Invalid AES-256-GCM key length".to_string());
            //     }
            //     crate::key::decrypt::decrypt_aes_256_gcm(&key_bytes, &self.uri.to_string())
            // } else {
            //     unimplemented!()
            // }

            unimplemented!(
                "Decryption logic is not fully implemented yet. This is a placeholder for the actual decryption process."
            );
        } else {
            return Err("No encryption key found".to_string());
        };
    }
}
