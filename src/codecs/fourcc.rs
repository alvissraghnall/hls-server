use crate::error::CodecParseError;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fourcc(u32);

impl From<u32> for Fourcc {
    fn from(fourcc: u32) -> Self {
        Self(fourcc)
    }
}

impl From<Fourcc> for u32 {
    fn from(fourcc: Fourcc) -> Self {
        fourcc.0
    }
}

impl From<&[u8; 4]> for Fourcc {
    fn from(n: &[u8; 4]) -> Self {
        Self(u32::from(n[0]) | u32::from(n[1]) << 8 | u32::from(n[2]) << 16 | u32::from(n[3]) << 24)
    }
}

impl From<Fourcc> for [u8; 4] {
    fn from(n: Fourcc) -> Self {
        [
            n.0 as u8,
            (n.0 >> 8) as u8,
            (n.0 >> 16) as u8,
            (n.0 >> 24) as u8,
        ]
    }
}

impl std::fmt::Display for Fourcc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c: [u8; 4] = (*self).into();

        f.write_fmt(format_args!(
            "{}{}{}{}",
            c[0] as char, c[1] as char, c[2] as char, c[3] as char
        ))
    }
}

impl std::fmt::Debug for Fourcc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:08x} ({})", self.0, self))
    }
}

impl Fourcc {
    #[must_use]
    pub fn new(bytes: &[u8; 4]) -> Self {
        Self::from(bytes)
    }

    /// Parse exactly 4 ASCII bytes into a Fourcc.
    pub(crate) fn of(s: &str) -> Result<Self, CodecParseError> {
        s.as_bytes()
            .try_into()
            .map(|b: &[u8; 4]| Self::new(b))
            .map_err(|_| CodecParseError::InvalidFourcc(s.to_string()))
    }
}

impl TryFrom<&str> for Fourcc {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let b = s.as_bytes();
        if b.len() != 4 {
            return Err("FourCC must be exactly 4 bytes");
        }
        Ok(Self::new(b.try_into().unwrap()))
    }
}
