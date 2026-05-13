use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri {
    scheme: String,
    authority: Option<String>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Uri {
    pub fn new(
        scheme: String,
        authority: Option<String>,
        path: String,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        Uri {
            scheme,
            authority,
            path,
            query,
            fragment,
        }
    }

    pub fn parse(s: &str) -> Result<Self, UriError> {
        let (scheme, rest) = s.split_once(':').ok_or(UriError::MissingScheme)?;
        if !rest.starts_with("//") {
            return Err(UriError::MissingAuthoritySeparator);
        }
        let rest = &rest[2..];

        let (authority, rest) = rest.split_once('/').unwrap_or((rest, ""));
        let authority = if authority.is_empty() {
            None
        } else {
            Some(authority.to_string())
        };

        let path_end = rest.find(|c| c == '?' || c == '#').unwrap_or(rest.len());
        let path = &rest[..path_end];
        let rest = &rest[path_end..];

        let (query, fragment) = match rest.chars().next() {
            Some('?') => {
                let rest = &rest[1..];
                let (q, f) = rest.split_once('#').unwrap_or((rest, ""));
                (
                    Some(q.to_string()),
                    if f.is_empty() {
                        None
                    } else {
                        Some(f.to_string())
                    },
                )
            }
            Some('#') => {
                let f = &rest[1..];
                (None, Some(f.to_string()))
            }
            _ => (None, None),
        };

        Ok(Uri {
            scheme: scheme.to_string(),
            authority,
            path: path.to_string(),
            query,
            fragment,
        })
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn authority(&self) -> Option<&str> {
        self.authority.as_deref()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    pub fn decoded_path(&self) -> Result<String, UriError> {
        decode(&self.path)
    }

    pub fn decoded_query(&self) -> Result<Option<String>, UriError> {
        self.query.as_ref().map(|q| decode(q)).transpose()
    }

    pub fn decoded_fragment(&self) -> Result<Option<String>, UriError> {
        self.fragment.as_ref().map(|f| decode(f)).transpose()
    }

    pub fn encode_component(s: &str) -> String {
        encode(s)
    }

    pub fn with_scheme(mut self, scheme: String) -> Self {
        self.scheme = scheme;
        self
    }

    pub fn with_authority(mut self, authority: Option<String>) -> Self {
        self.authority = authority;
        self
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }

    pub fn with_query(mut self, query: Option<String>) -> Self {
        self.query = query;
        self
    }

    pub fn with_fragment(mut self, fragment: Option<String>) -> Self {
        self.fragment = fragment;
        self
    }
}

impl Display for Uri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:", self.scheme)?;
        if let Some(ref auth) = self.authority {
            write!(f, "//{}", auth)?;
        }
        write!(f, "{}", self.path)?;
        if let Some(ref query) = self.query {
            write!(f, "?{}", query)?;
        }
        if let Some(ref fragment) = self.fragment {
            write!(f, "#{}", fragment)?;
        }
        Ok(())
    }
}

impl FromStr for Uri {
    type Err = UriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uri::parse(s)
    }
}

fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

/// encodes a string by replacing non-unreserved characters with %XX sequences.
fn encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        if is_unreserved(*byte) {
            result.push(*byte as char);
        } else {
            result.push('%');
            result.push(hex_char(*byte >> 4));
            result.push(hex_char(*byte & 0x0F));
        }
    }
    result
}

/// decodes a percent-encoded string.
fn decode(input: &str) -> Result<String, UriError> {
    let mut result = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            if i + 2 >= bytes.len() {
                return Err(UriError::InvalidPercentEncoding);
            }
            let high = hex_to_int(bytes[i + 1])?;
            let low = hex_to_int(bytes[i + 2])?;
            result.push(high << 4 | low);
            i += 3;
        } else {
            // Push valid utf8 byte directly
            result.push(b);
            i += 1;
        }
    }

    String::from_utf8(result).map_err(|_| UriError::InvalidUtf8)
}

fn hex_char(b: u8) -> char {
    match b {
        0..=9 => (b'0' + b) as char,
        10..=15 => (b'a' + b - 10) as char,
        _ => unreachable!(),
    }
}

fn hex_to_int(b: u8) -> Result<u8, UriError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(UriError::InvalidPercentEncoding),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriError {
    MissingScheme,
    MissingAuthoritySeparator,
    InvalidFormat,
    InvalidPercentEncoding,
    InvalidUtf8,
}

impl Display for UriError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UriError::MissingScheme => write!(f, "URI missing scheme"),
            UriError::MissingAuthoritySeparator => {
                write!(f, "URI missing '//' after scheme")
            }
            UriError::InvalidFormat => write!(f, "URI has invalid format"),
            UriError::InvalidPercentEncoding => write!(f, "Invalid percent-encoded sequence"),
            UriError::InvalidUtf8 => write!(f, "Decoded bytes are not valid UTF-8"),
        }
    }
}

impl std::error::Error for UriError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple() {
        let uri = Uri::parse("https://docs.rs/hello%20world").unwrap();
        assert_eq!(uri.path(), "/hello%20world");
        assert_eq!(uri.decoded_path().unwrap(), "/hello world");
    }

    #[test]
    fn test_decode_query() {
        let uri = Uri::parse("https://cargo.rs/?a=1&b=2%2B2").unwrap();
        assert_eq!(uri.query(), Some("a=1&b=2%2B2"));
        assert_eq!(
            uri.decoded_query().unwrap(),
            Some(String::from("a=1&b=2+2"))
        );
    }

    #[test]
    fn test_encode() {
        let raw = "/path with spaces&symbols#";
        let encoded = Uri::encode_component(raw);
        assert_eq!(encoded, "/path%20with%20spaces%26symbols%23");

        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_invalid_percent() {
        let uri = Uri::parse("https://docs.rs/%ZZ").unwrap();
        assert!(matches!(
            uri.decoded_path(),
            Err(UriError::InvalidPercentEncoding)
        ));
    }

    #[test]
    fn test_display_encoded() {
        let uri = Uri::new(
            "https".to_string(),
            Some("docs.rs".to_string()),
            "/a b".to_string(),
            None,
            None,
        );
        assert_eq!(uri.to_string(), "https://docs.rs/a b");

        let safe_path = Uri::encode_component("/a b");
        let uri_safe = Uri::new(
            "https".to_string(),
            Some("docs.rs".to_string()),
            safe_path,
            None,
            None,
        );
        assert_eq!(uri_safe.to_string(), "https://docs.rs/a%20b");
    }
}
