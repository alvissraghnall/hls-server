use std::str::FromStr;

use crate::{codecs::{DolbyVision, DolbyVisionBase, SupplementalCodecEntry, fourcc::Fourcc}, error::{CodecParseError, SupplementalCodecParseError}};

use super::{
    AvcCodec, AvcProfile, Codec, GenericCodec, Hevc, HevcProfile, HevcTier, Mp4aCodec, Mp4vCodec,
    Vp9, Vp9Profile, VpChromaSubsampling,
};

impl AvcProfile {
    /// RECONSTRUCT the profile variant from the raw (`profile_idc`, `constraint_byte`)
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

impl FromStr for DolbyVision {
    type Err = CodecParseError;
 
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
 
        let fourcc_str = parts.next().ok_or(CodecParseError::Empty)?;
        let base = DolbyVisionBase::from_fourcc(fourcc_str)
            .ok_or_else(|| CodecParseError::InvalidFourcc(fourcc_str.to_owned()))?;
 
        let profile = parts
            .next()
            .ok_or(CodecParseError::MissingField("dv_profile"))?
            .parse::<u8>()
            .map_err(|e| CodecParseError::InvalidDecimal { field: "dv_profile", inner: e })?;
 
        let level = parts
            .next()
            .ok_or(CodecParseError::MissingField("dv_level"))?
            .parse::<u8>()
            .map_err(|e| CodecParseError::InvalidDecimal { field: "dv_level", inner: e })?;
 
        Ok(DolbyVision { base, profile, level })
    }
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
        let level = level_byte & 0x7F;

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


impl FromStr for SupplementalCodecEntry {
    type Err = SupplementalCodecParseError;
 
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split on '/' — first token is the codec, rest are brands.
        let mut fields = s.split('/');
 
        let codec_str = fields.next().ok_or(SupplementalCodecParseError::Empty)?;
        let codec = codec_str
            .parse::<Codec>()
            .map_err(SupplementalCodecParseError::Codec)?;
 
        let brands = fields
            .map(|brand_str| {
                Fourcc::of(brand_str)
                    .map_err(|_| SupplementalCodecParseError::InvalidBrand(brand_str.to_owned()))
            })
            .collect::<Result<Vec<Fourcc>, _>>()?;
 
        Ok(SupplementalCodecEntry::new(codec, brands))
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
            "dvh1" | "dvhe" | "dvav" | "dva1" => s.parse().map(Codec::DolbyVision),
            _ => s.parse().map(Codec::Unknown),
        }
    }
}

pub(crate) fn parse_codecs_attr(s: &str) -> Result<Vec<Codec>, (usize, CodecParseError)> {
    s.split(',')
        .enumerate()
        .map(|(i, part)| part.trim().parse().map_err(|e| (i, e)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avc_constrained_baseline_3_0() {
        let s = "avc1.42401e";
        // 0x42 + constraint bit 6 set
        let c: Codec = s.parse().unwrap();
        assert_eq!(
            c,
            Codec::Avc(AvcCodec {
                fourcc: Fourcc::new(b"avc1"),
                profile: AvcProfile::ConstrainedBaseline,
                level: 30,
            })
        );
        assert_eq!(c.to_string(), s);
    }

    #[test]
    fn avc_high_4_0() {
        let s = "avc1.640028";
        let c: Codec = s.parse().unwrap();
        assert_eq!(
            c,
            Codec::Avc(AvcCodec {
                fourcc: Fourcc::new(b"avc1"),
                profile: AvcProfile::High,
                level: 40,
            })
        );
        assert_eq!(c.to_string(), s);
    }

    #[test]
    fn avc_constrained_high() {
        // 0x64 + bits 2+3 set (0x0C)
        let c: Codec = "avc1.640c1f".parse().unwrap();
        assert_eq!(
            c,
            Codec::Avc(AvcCodec {
                fourcc: Fourcc::new(b"avc1"),
                profile: AvcProfile::ConstrainedHigh,
                level: 31,
            })
        );
    }

    #[test]
    fn avc_alt_fourcc() {
        let c: Codec = "avc3.640028".parse().unwrap();
        if let Codec::Avc(avc) = c {
            assert_eq!(avc.fourcc, Fourcc::new(b"avc3"));
            assert_eq!(avc.profile, AvcProfile::High);
        } else {
            panic!("expected Avc");
        }
    }

    #[test]
    fn avc_wrong_hex_length() {
        assert!("avc1.4d40".parse::<Codec>().is_err());
        assert!("avc1.4d401e00".parse::<Codec>().is_err());
    }

    #[test]
    fn mp4a_aac_lc() {
        let s = "mp4a.40.2";
        let c: Codec = s.parse().unwrap();
        assert_eq!(
            c,
            Codec::Mp4a(Mp4aCodec {
                oti: 0x40,
                audio_object_type: Some(2)
            })
        );
        assert_eq!(c.to_string(), s);
    }

    #[test]
    fn mp4a_no_aot() {
        let s = "mp4a.40";
        let c: Codec = s.parse().unwrap();
        assert_eq!(
            c,
            Codec::Mp4a(Mp4aCodec {
                oti: 0x40,
                audio_object_type: None
            })
        );
        assert_eq!(c.to_string(), s);
    }

    #[test]
    fn vp9_profile0_level31() {
        let c: Codec = "vp09.00.31.08.01".parse().unwrap();
        assert_eq!(
            c,
            Codec::Vp9(Vp9 {
                profile: Vp9Profile::Profile0,
                level: 31,
                bit_depth: 8,
                chroma: VpChromaSubsampling::Horizontal,
            })
        );
    }

    #[test]
    fn vp9_profile2_10bit() {
        let c: Codec = "vp09.02.51.10.00".parse().unwrap();
        if let Codec::Vp9(v) = c {
            assert_eq!(v.profile, Vp9Profile::Profile2);
            assert_eq!(v.bit_depth, 10);
            assert_eq!(v.chroma, VpChromaSubsampling::Vertical);
        } else {
            panic!("expected Vp9");
        }
    }

    #[test]
    fn vp9_bad_profile() {
        assert!("vp09.09.31.08.01".parse::<Codec>().is_err());
    }

    #[test]
    fn vp9_bad_chroma() {
        assert!("vp09.00.31.08.09".parse::<Codec>().is_err());
    }

    #[test]
    fn hevc_main_main_tier() {
        // profile_idc=1 (Main), level_byte=0x5d=93, bit7=0 -> Main tier, level=3.1
        let c: Codec = "hev1.01.5d".parse().unwrap();
        if let Codec::Hevc(h) = c {
            assert_eq!(h.profile, HevcProfile::Main);
            assert_eq!(h.tier, HevcTier::Main);
            assert!(
                ((h.level as f32 / 30.0) - 3.1).abs() < 0.01,
                "level was {}",
                h.level
            );
            assert!(h.constraint.is_none());
        } else {
            panic!("expected Hevc");
        }
    }

    #[test]
    fn hevc_main10_high_tier() {
        // bit7=1 -> High tier, level = (0xdd & 0x7f) / 30.0 = 93/30 = 3.1
        let c: Codec = "hvc1.02.dd".parse().unwrap();
        if let Codec::Hevc(h) = c {
            assert_eq!(h.profile, HevcProfile::Main10);
            assert_eq!(h.tier, HevcTier::High);
            let level_float = h.level as f32 / 30.0;
            assert!(
                (level_float - 3.1).abs() < 0.01,
                "level was {}, expected = {}",
                h.level,
                level_float
            );
        } else {
            panic!("expected Hevc");
        }
    }

    #[test]
    fn hevc_unknown_profile() {
        assert!("hev1.ff.5d".parse::<Codec>().is_err());
    }

    #[test]
    fn mp4v_with_level() {
        let c: Codec = "mp4v.20.9".parse().unwrap();
        assert_eq!(
            c,
            Codec::Mp4v(Mp4vCodec {
                oti: 0x20,
                profile_level_indication: Some(9)
            })
        );
    }

    #[test]
    fn mp4v_no_level() {
        let c: Codec = "mp4v.20".parse().unwrap();
        assert_eq!(
            c,
            Codec::Mp4v(Mp4vCodec {
                oti: 0x20,
                profile_level_indication: None
            })
        );
    }

    #[test]
    fn generic() {
        let c: Codec = "ec-3.some.thing".parse().unwrap();
        assert!(matches!(c, Codec::Unknown(_)));
    }

    #[test]
    fn empty_is_err() {
        assert!("".parse::<Codec>().is_err());
    }

    #[test]
    fn case_insensitive_dispatch() {
        // Kind matching is case-insensitive...fourcc itself is preserved.
        let lower: Codec = "mp4a.40.2".parse().unwrap();
        let upper: Codec = "MP4A.40.2".parse().unwrap();
        assert_eq!(lower, upper);
    }

    #[test]
    fn codecs_attr_avc_aac() {
        let codecs = parse_codecs_attr("avc1.640028,mp4a.40.2").unwrap();
        assert_eq!(codecs.len(), 2);
        assert!(matches!(codecs[0], Codec::Avc(_)));
        assert!(matches!(codecs[1], Codec::Mp4a(_)));
    }

    #[test]
    fn codecs_attr_trims_whitespace() {
        let codecs = parse_codecs_attr("avc1.640028 , mp4a.40.2").unwrap();
        assert_eq!(codecs.len(), 2);
    }
  

}
