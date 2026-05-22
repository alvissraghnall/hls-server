use std::str::FromStr;

use crate::codecs::fourcc::Fourcc;

use super::{
    AvcCodec, AvcProfile, Codec, GenericCodec, Hevc, HevcProfile, HevcTier, Mp4aCodec, Mp4vCodec,
    Vp9, Vp9Profile, VpChromaSubsampling,
};

#[derive(Debug)]
pub enum CodecParseError {
    Empty,
    MissingField(&'static str),
    InvalidHex {
        field: &'static str,
        inner: std::num::ParseIntError,
    },
    InvalidDecimal {
        field: &'static str,
        inner: std::num::ParseIntError,
    },
    UnknownAvcProfile(u8),
    UnknownHevcProfile(u8),
    UnknownVp9Profile(u8),
    InvalidChroma(u8),
    InvalidFourcc(String),
    WrongHexLength {
        field: &'static str,
        expected: usize,
        got: usize,
    },
}

impl std::fmt::Display for CodecParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty codec string"),
            Self::MissingField(name) => write!(f, "missing required field: {name}"),
            Self::InvalidHex { field, inner } => {
                write!(f, "invalid hex in field '{field}': {inner}")
            }
            Self::InvalidDecimal { field, inner } => {
                write!(f, "invalid decimal in field '{field}': {inner}")
            }
            Self::UnknownAvcProfile(p) => write!(f, "unknown AVC profile IDC 0x{p:02x}"),
            Self::UnknownHevcProfile(p) => write!(f, "unknown HEVC profile IDC {p}"),
            Self::UnknownVp9Profile(p) => write!(f, "unknown VP9 profile {p}"),
            Self::InvalidChroma(c) => write!(f, "invalid VP chroma subsampling value {c}"),
            Self::InvalidFourcc(s) => write!(f, "FourCC must be 4 ASCII bytes, got {s:?}"),
            Self::WrongHexLength {
                field,
                expected,
                got,
            } => write!(f, "field '{field}' must be {expected} hex chars, got {got}"),
        }
    }
}

impl std::error::Error for CodecParseError {}

impl AvcProfile {
    /// RECONSTRUCT the profile variant from the raw (profile_idc, constraint_byte)
    /// pair encoded in the codec string.  constraint flags are matched in
    /// precedence order per ITU-T H.264 Annex A.
    pub(crate) fn from_idc(p: u8, c: u8) -> Result<Self, CodecParseError> {
        Ok(match (p, c) {
            (0x42, c) if c & 0x40 != 0 => Self::ConstrainedBaseline,
            (0x42, _) => Self::Baseline,
            (0x4D, _) => Self::Main,
            (0x58, _) => Self::Extended,
            (0x64, c) if c & 0x0C == 0x0C => Self::ConstrainedHigh,
            (0x64, c) if c & 0x08 != 0 => Self::ProgressiveHigh,
            (0x64, _) => Self::High,
            (0x6E, c) if c & 0x10 != 0 => Self::High10Intra,
            (0x6E, _) => Self::High10,
            (0x7A, c) if c & 0x10 != 0 => Self::High422Intra,
            (0x7A, _) => Self::High422,
            (0xF4, c) if c & 0x10 != 0 => Self::High444Intra,
            (0xF4, _) => Self::High444Predictive,
            (0x2C, _) => Self::CAVLC444Intra,
            (0x53, c) if c & 0x04 != 0 => Self::ScalableConstrainedBaseline,
            (0x53, _) => Self::ScalableBaseline,
            (0x56, c) if c & 0x10 != 0 => Self::ScalableHighIntra,
            (0x56, c) if c & 0x04 != 0 => Self::ScalableConstrainedHigh,
            (0x56, _) => Self::ScalableHigh,
            (0x80, _) => Self::StereoHigh,
            (0x76, _) => Self::MultiviewHigh,
            (0x8A, _) => Self::MultiviewDepthHigh,
            _ => return Err(CodecParseError::UnknownAvcProfile(p)),
        })
    }
}

impl HevcProfile {
    pub(crate) fn from_idc(idc: u8) -> Result<Self, CodecParseError> {
        Ok(match idc {
            1 => Self::Main,
            2 => Self::Main10,
            3 => Self::MainStillPicture,
            4 => Self::RangeExtensions,
            5 => Self::HighThroughput,
            6 => Self::MultiviewMain,
            7 => Self::ScalableMain,
            8 => Self::ThreeDMain,
            9 => Self::ScreenExtended,
            10 => Self::ScalableRangeExtensions,
            11 => Self::HighThroughputScreenExtended,
            _ => return Err(CodecParseError::UnknownHevcProfile(idc)),
        })
    }
}

impl TryFrom<u8> for Vp9Profile {
    type Error = CodecParseError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        Ok(match n {
            0 => Self::Profile0,
            1 => Self::Profile1,
            2 => Self::Profile2,
            3 => Self::Profile3,
            _ => return Err(CodecParseError::UnknownVp9Profile(n)),
        })
    }
}

impl TryFrom<u8> for VpChromaSubsampling {
    type Error = CodecParseError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        Ok(match n {
            0 => Self::Vertical,
            1 => Self::Horizontal,
            2 => Self::Reserved,
            3 => Self::None,
            _ => return Err(CodecParseError::InvalidChroma(n)),
        })
    }
}

fn hex_u8(s: &str, field: &'static str) -> Result<u8, CodecParseError> {
    u8::from_str_radix(s, 16).map_err(|inner| CodecParseError::InvalidHex { field, inner })
}

fn hex_u16(s: &str, field: &'static str) -> Result<u16, CodecParseError> {
    u16::from_str_radix(s, 16).map_err(|inner| CodecParseError::InvalidHex { field, inner })
}

fn hex_u32(s: &str, field: &'static str) -> Result<u32, CodecParseError> {
    u32::from_str_radix(s, 16).map_err(|inner| CodecParseError::InvalidHex { field, inner })
}

fn dec_u8(s: &str, field: &'static str) -> Result<u8, CodecParseError> {
    s.parse()
        .map_err(|inner| CodecParseError::InvalidDecimal { field, inner })
}

fn dec_u16(s: &str, field: &'static str) -> Result<u16, CodecParseError> {
    s.parse()
        .map_err(|inner| CodecParseError::InvalidDecimal { field, inner })
}

fn dec_u32(s: &str, field: &'static str) -> Result<u32, CodecParseError> {
    s.parse()
        .map_err(|inner| CodecParseError::InvalidDecimal { field, inner })
}

// mp4v.[hex-oti].[decimal-profile-level?]
impl FromStr for Mp4aCodec {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, '.');
        let _ = parts.next(); // "mp4a"
        let oti = hex_u16(
            parts.next().ok_or(CodecParseError::MissingField("oti"))?,
            "oti",
        )?;
        let audio_object_type = parts
            .next()
            .map(|p| dec_u16(p, "audio_object_type"))
            .transpose()?;
        Ok(Mp4aCodec {
            oti,
            audio_object_type,
        })
    }
}

// mp4v.[hex-oti].[decimal-profile-level?]
impl FromStr for Mp4vCodec {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, '.');
        let _ = parts.next(); // "mp4v"
        let oti = hex_u16(
            parts.next().ok_or(CodecParseError::MissingField("oti"))?,
            "oti",
        )?;
        let profile_level_indication = parts
            .next()
            .map(|p| dec_u16(p, "profile_level_indication"))
            .transpose()?;
        Ok(Mp4vCodec {
            oti,
            profile_level_indication,
        })
    }
}

// [avc1|avc2|avc3].PPCCLL
impl FromStr for AvcCodec {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (fourcc_str, hex_str) = s
            .split_once('.')
            .ok_or(CodecParseError::MissingField("profile+constraint+level"))?;

        if hex_str.len() != 6 {
            return Err(CodecParseError::WrongHexLength {
                field: "avc profile+constraint+level",
                expected: 6,
                got: hex_str.len(),
            });
        }

        let fourcc = Fourcc::of(fourcc_str)?;
        let profile_idc = hex_u8(&hex_str[0..2], "profile_idc")?;
        let constraint = hex_u8(&hex_str[2..4], "constraint_byte")?;
        let level_byte = hex_u8(&hex_str[4..6], "level")?;
        let profile = AvcProfile::from_idc(profile_idc, constraint)?;
        let level = level_byte;

        Ok(AvcCodec {
            fourcc,
            profile,
            level,
        })
    }
}

// [hev1|hvc1].[profile-idc:hex].[tier+level:hex][.[constraint-4cc].[constraint-val:hex]]
impl FromStr for Hevc {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');

        let fourcc_str = parts.next().ok_or(CodecParseError::Empty)?;
        let profile_str = parts
            .next()
            .ok_or(CodecParseError::MissingField("profile_idc"))?;
        let level_str = parts
            .next()
            .ok_or(CodecParseError::MissingField("tier+level"))?;

        let fourcc = Fourcc::of(fourcc_str)?;
        let profile = HevcProfile::from_idc(hex_u8(profile_str, "profile_idc")?)?;

        let level_byte = hex_u8(level_str, "tier+level")?;
        let tier = if level_byte & 0x80 != 0 {
            HevcTier::High
        } else {
            HevcTier::Main
        };
        let level = (level_byte & 0x7F);

        // both fields gotta be present or neither.
        let constraint = match (parts.next(), parts.next()) {
            (Some(c_str), Some(v_str)) => {
                Some((Fourcc::of(c_str)?, hex_u32(v_str, "constraint_val")?))
            }
            (None, None) => None,
            _ => {
                return Err(CodecParseError::MissingField(
                    "constraint requires both a 4CC and a value",
                ));
            }
        };

        Ok(Hevc {
            profile,
            tier,
            level,
            constraint,
            fourcc,
        })
    }
}

// vp09.[profile].[level].[bit-depth].[chroma] ::: all zero-padded decimal
impl FromStr for Vp9 {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let _ = parts.next(); // "vp09"

        let profile = Vp9Profile::try_from(dec_u8(
            parts
                .next()
                .ok_or(CodecParseError::MissingField("profile"))?,
            "profile",
        )?)?;
        let level = dec_u8(
            parts.next().ok_or(CodecParseError::MissingField("level"))?,
            "level",
        )?;
        let bit_depth = dec_u8(
            parts
                .next()
                .ok_or(CodecParseError::MissingField("bit_depth"))?,
            "bit_depth",
        )?;
        let chroma = VpChromaSubsampling::try_from(dec_u8(
            parts
                .next()
                .ok_or(CodecParseError::MissingField("chroma"))?,
            "chroma",
        )?)?;

        Ok(Vp9 {
            profile,
            level,
            bit_depth,
            chroma,
        })
    }
}

impl FromStr for GenericCodec {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CodecParseError::Empty);
        }
        Ok(GenericCodec {
            parts: s.split('.').map(str::to_owned).collect(),
        })
    }
}

impl FromStr for Codec {
    type Err = CodecParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CodecParseError::Empty);
        }

        let kind = s.split('.').next().unwrap().to_ascii_lowercase();

        match kind.as_str() {
            "mp4a" => s.parse().map(Codec::Mp4a),
            "mp4v" => s.parse().map(Codec::Mp4v),
            k if k.starts_with("avc") => s.parse().map(Codec::Avc),
            "hev1" | "hvc1" | "hev2" => s.parse().map(Codec::Hevc),
            "vp09" | "vp9" => s.parse().map(Codec::Vp9),
            _ => s.parse().map(Codec::Unknown),
        }
    }
}

pub fn parse_codecs_attr(s: &str) -> Result<Vec<Codec>, (usize, CodecParseError)> {
    s.split(',')
        .enumerate()
        .map(|(i, part)| part.trim().parse().map_err(|e| (i, e)))
        .collect()
}
