use std::{fmt::{self, LowerHex}};

use crate::codecs::fourcc::Fourcc;

pub mod fourcc;
pub mod parse;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Codec {
    Mp4a(Mp4aCodec),
    Mp4v(Mp4vCodec),
    Avc(AvcCodec),
    Hevc(Hevc),
    Vp9(Vp9),
    Unknown(GenericCodec),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Vp9 {
    profile: Vp9Profile,
    level: u8, // level * 10 (e.g. 1.0 -> 10, 3.1 -> 31)
    bit_depth: u8,
    chroma: VpChromaSubsampling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mp4aCodec {
    // mp4a.40.2
    oti: u16,                       // 0x40
    audio_object_type: Option<u16>, // 2 = AAC-LC
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mp4vCodec {
    // mp4v.20.9
    oti: u16, // 0x20
    profile_level_indication: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AvcCodec {
    // avc1.4d401e
    // Added fourcc to support avc1, avc2, avc3, etc.
    fourcc: Fourcc,
    profile: AvcProfile,
    level: u8, // level * 10 (e.g. 1.0 -> 10, 3.1 -> 31)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenericCodec {
    parts: Vec<String>,
}

impl Mp4aCodec {
    pub const KIND: &str = "mp4a";
}

impl Mp4vCodec {
    pub const KIND: &str = "mp4v";
}

impl AvcCodec {
}

impl Vp9 {
    pub const KIND: &str = "vp09";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvcProfile {
    ConstrainedBaseline,
    Baseline,
    Main,
    Extended,
    High,
    ProgressiveHigh,
    ConstrainedHigh,
    High10,
    High422,
    High444Predictive,
    High10Intra,
    High422Intra,
    High444Intra,
    CAVLC444Intra,
    ScalableBaseline,
    ScalableConstrainedBaseline,
    ScalableHigh,
    ScalableConstrainedHigh,
    ScalableHighIntra,
    StereoHigh,
    MultiviewHigh,
    MultiviewDepthHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HevcProfile {
    Main,
    Main10,
    MainStillPicture,
    RangeExtensions,
    HighThroughput,
    MultiviewMain,
    ScalableMain,
    ThreeDMain,
    ScreenExtended,
    ScalableRangeExtensions,
    HighThroughputScreenExtended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HevcTier {
    Main, // 0
    High, // 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpChromaSubsampling {
    Vertical = 0,   // 4:0:0
    Horizontal = 1, // 4:2:0
    Reserved = 2,   // 4:2:2
    None = 3,       // 4:4:4
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hevc {
    profile: HevcProfile,
    tier: HevcTier,
    level: u8, // level * 30 
    constraint: Option<(Fourcc, u32)>, // (4CC, val)
    fourcc: Fourcc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vp9Profile {
    Profile0,
    Profile1,
    Profile2,
    Profile3,
}

impl AvcProfile {
    /// (profile_idc_hex, constraint_byte_hex)
    /// based on ITU-T H.264 Annex A
    pub fn hex_codes(&self) -> (u8, u8) {
        match self {
            AvcProfile::ConstrainedBaseline => (0x42, 0x40), // 66, set1
            AvcProfile::Baseline => (0x42, 0x00),            // 66
            AvcProfile::Main => (0x4D, 0x00),                // 77
            AvcProfile::Extended => (0x58, 0x00),            // 88
            AvcProfile::High => (0x64, 0x00),                // 100
            AvcProfile::ProgressiveHigh => (0x64, 0x08),     // 100, set4
            AvcProfile::ConstrainedHigh => (0x64, 0x0C),     // 100, set4+5
            AvcProfile::High10 => (0x6E, 0x00),              // 110
            AvcProfile::High422 => (0x7A, 0x00),             // 122
            AvcProfile::High444Predictive => (0xF4, 0x00),   // 244
            AvcProfile::High10Intra => (0x6E, 0x10),         // 110, set3
            AvcProfile::High422Intra => (0x7A, 0x10),        // 122, set3
            AvcProfile::High444Intra => (0xF4, 0x10),        // 244, set3
            AvcProfile::CAVLC444Intra => (0x2C, 0x00),       // 44
            AvcProfile::ScalableBaseline => (0x53, 0x00),    // 83
            AvcProfile::ScalableConstrainedBaseline => (0x53, 0x04), // 83, set5
            AvcProfile::ScalableHigh => (0x56, 0x00),        // 86
            AvcProfile::ScalableConstrainedHigh => (0x56, 0x04), // 86, set5
            AvcProfile::ScalableHighIntra => (0x56, 0x10),   // 86, set3
            AvcProfile::StereoHigh => (0x80, 0x00),          // 128
            AvcProfile::MultiviewHigh => (0x76, 0x00),       // 118
            AvcProfile::MultiviewDepthHigh => (0x8A, 0x00),  // 138
        }
    }
}

impl HevcProfile {
    fn profile_idc(&self) -> u8 {
        match self {
            HevcProfile::Main => 1,
            HevcProfile::Main10 => 2,
            HevcProfile::MainStillPicture => 3,
            HevcProfile::RangeExtensions => 4,
            HevcProfile::HighThroughput => 5,
            HevcProfile::MultiviewMain => 6,
            HevcProfile::ScalableMain => 7,
            HevcProfile::ThreeDMain => 8,
            HevcProfile::ScreenExtended => 9,
            HevcProfile::ScalableRangeExtensions => 10,
            HevcProfile::HighThroughputScreenExtended => 11,
        }
    }
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // mp4a.[hex-oti].[decimal-audio-oti]
            Codec::Mp4a(mp4a) => {
                write!(f, "{}.{:02x}", Mp4aCodec::KIND, mp4a.oti)?;
                if let Some(aot) = mp4a.audio_object_type {
                    write!(f, ".{}", aot)?; // Decimal as per RFC 6381
                }
                Ok(())
            }

            // mp4v.[hex-oti].[decimal-profile-level]
            Codec::Mp4v(mp4v) => {
                if mp4v.profile_level_indication.is_none() {
                    write!(f, "{}.{:02x}", Mp4vCodec::KIND, mp4v.oti)
                } else {
                    write!(
                        f,
                        "{}.{:02x}.{}",
                        Mp4vCodec::KIND,
                        mp4v.oti,
                        mp4v.profile_level_indication.unwrap()
                    )
                }
            }

            // avc1.[profile][constraint][level] (Concatenated Hex)
            Codec::Avc(avc) => {
                let (p, c) = avc.profile.hex_codes();
                let l = avc.level;
                // Use the stored fourcc instead of hardcoded "avc1"
                write!(f, "{}.{:02x}{:02x}{:02x}", avc.fourcc, p, c, l)
            }

            // hev1.[profile].[tier+level].[constraint]?
            // Level calc: decimal * 30.
            // Byte: (level) | (is_high_tier << 7)
            Codec::Hevc(hevc) => {
                let p = hevc.profile.profile_idc();
                let level_val = hevc.level;
                let tier_bit = if hevc.tier == HevcTier::High {
                    0x80
                } else {
                    0x00
                };
                let level_byte = level_val | tier_bit;

                write!(f, "{}.{:02x}.{:02x}", hevc.fourcc, p, level_byte)?;

                if let Some((c_id, c_val)) = &hevc.constraint {
                    // constraint bytes must be hex (e.g. "L93.B0")
                    write!(f, ".{}.{:08x}", c_id, c_val)?;
                }
                Ok(())
            }

            // vp09.[profile].[level].[bit_depth].[chroma]
            Codec::Vp9(vp9) => {
                let p = vp9.profile as u8;
                let l = vp9.level;

                // write!(
                //     f,
                //     "{}.{:02x}.{:02x}.{:02x}.{:02x}",
                //     Vp9::KIND,
                //     p,
                //     l,
                //     vp9.bit_depth,
                //     vp9.chroma
                // )

                write!(
                    f,
                    "{}.{:02}.{:02}.{:02}.{:02}",
                    Vp9::KIND,
                    p,
                    l,
                    vp9.bit_depth,
                    vp9.chroma
                )
            }

            Codec::Unknown(generic) => write!(f, "{}", generic.parts.join(".")),
        }
    }
}

impl LowerHex for VpChromaSubsampling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}", *self as u8)
    }
}

impl std::fmt::Display for VpChromaSubsampling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}", *self as u8)
    }
}
