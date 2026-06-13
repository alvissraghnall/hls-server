use std::str::FromStr;

use crate::{
    CRLF,
    attribute_list::{AttributeList, AttributeValue, parse_attribute_list},
    error::{ParseError, ValidationError},
    playlist::{PlayListVariableDefinition, SharedTag},
};

pub trait Tag {
    // fn from_str(s: &str) -> Result<Self, ParseError>
    // where
    //     Self: Sized;

    fn validate(&self) -> Result<(), ValidationError>;
}

impl FromStr for SharedTag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_shared_tag(s, 0)
    }
}

pub(crate) fn parse_shared_tag(line: &str, line_number: usize) -> Result<SharedTag, ParseError> {
    match line {
        s if s.starts_with("#EXT-X-VERSION:") => {
            let version = s
                .strip_prefix("#EXT-X-VERSION:")
                .and_then(|v| v.parse::<u8>().ok());

            if let Some(v) = version {
                return Ok(SharedTag::Version(v));
            } else {
                return Err(ParseError::InvalidLine(format!(
                    "{} is not valid according to HLS spec.",
                    line
                )));
            }
        }
        "#EXT-X-INDEPENDENT-SEGMENTS" => Ok(SharedTag::IndependentSegments),
        s if s.starts_with("#EXT-X-START:") => {
            let s = s
                .strip_prefix("#EXT-X-START:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{} is not valid according to HLS spec.",
                    line
                )))?;
            let attrs = parse_attribute_list(s)?;

            let time_offset = attrs
                .get("TIME-OFFSET")
                .and_then(|v| v.as_signed_decimal_floating_point())
                .ok_or(ParseError::InvalidAttributeValue {
                    attribute: "TIME-OFFSET".into(),
                    value: "NONE".into(),
                    expected: "a valid signed decimal floating point number".into(),
                })?;
            let precise = attrs
                .get("PRECISE")
                .and_then(|v| v.as_enumerated_string())
                .map(|v| v == "YES")
                .unwrap_or(false);
            Ok(SharedTag::Start {
                precise,
                time_offset,
            })
        }
        s if s.starts_with("#EXT-X-DEFINE:") => {
            let attrs = s
                .strip_prefix("#EXT-X-DEFINE:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{} is not valid according to HLS spec.",
                    line
                )))?;
            let attrs = parse_attribute_list(attrs)?;

            let var = PlayListVariableDefinition::try_from(attrs)?;

            Ok(SharedTag::Variable(var))
        }
        _ => Err(ParseError::UnknownTag {
            tag: line.into(),
            span: crate::error::Span {
                line: line_number,
                column: 0,
            }, // change sooon x
        }),
    }
}

impl ToString for SharedTag {
    fn to_string(&self) -> String {
        match self {
            SharedTag::Version(v) => format!("#EXT-X-VERSION:{}", v),
            SharedTag::IndependentSegments => format!("#EXT-X-INDEPENDENT-SEGMENTS"),
            SharedTag::Start {
                precise,
                time_offset,
            } => format!(
                "#EXT-X-START:PRECISE={},TIME-OFFSET={}",
                precise, time_offset
            ),
            SharedTag::Variable(var) => format!("#EXT-X-DEFINE:{}", var),
        }
    }
}

impl Tag for SharedTag {
    fn validate(&self) -> Result<(), ValidationError> {
        match self {
            // cant do this here because we have to check other
            // tags independent of this one. ideally, we do this in
            // Playlist impl block. let's see.
            SharedTag::Version(_) => {}
            SharedTag::Variable(play_list_variable_definition) => todo!(),
            SharedTag::IndependentSegments => todo!(),
            SharedTag::Start {
                precise,
                time_offset,
            } => todo!(),
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{attribute_list::parse_quoted_string, playlist::PlayListVariableDefinition};

    #[test]
    fn test_parse_version() {
        let tag = parse_shared_tag("#EXT-X-VERSION:3", 3).unwrap();
        assert_eq!(tag, SharedTag::Version(3));

        assert!(parse_shared_tag("#EXT-X-VERSION:abc", 3).is_err());
    }

    #[test]
    fn test_parse_independent_segments() {
        let tag = parse_shared_tag("#EXT-X-INDEPENDENT-SEGMENTS", 3).unwrap();
        assert_eq!(tag, SharedTag::IndependentSegments);
    }

    #[test]
    fn test_parse_start() {
        let tag = parse_shared_tag("#EXT-X-START:TIME-OFFSET=10.5,PRECISE=YES", 3).unwrap();
        assert_eq!(
            tag,
            SharedTag::Start {
                precise: true,
                time_offset: 10.5
            }
        );

        let tag = parse_shared_tag("#EXT-X-START:TIME-OFFSET=-2.0", 3).unwrap();
        assert_eq!(
            tag,
            SharedTag::Start {
                precise: false,
                time_offset: -2.0
            }
        );
    }

    #[test]
    fn test_parse_define_name_value() {
        let tag = parse_shared_tag("#EXT-X-DEFINE:NAME=\"VAR\",VALUE=\"val\"", 3).unwrap();
        if let SharedTag::Variable(PlayListVariableDefinition::NameValue { name, value }) = tag {
            assert_eq!(name, "VAR");
            assert_eq!(value, "val");
        } else {
            panic!("Expected NameValue");
        }
    }

    #[test]
    fn test_parse_define_import() {
        let tag = parse_shared_tag("#EXT-X-DEFINE:IMPORT=\"VAR\"", 3).unwrap();
        if let SharedTag::Variable(PlayListVariableDefinition::Import { name }) = tag {
            assert_eq!(name, "VAR");
        } else {
            panic!("Expected Import");
        }
    }

    #[test]
    fn test_parse_attribute_list() {
        let attrs = parse_attribute_list("NAME=\"VAR\",VALUE=\"val\",BANDWIDTH=1000").unwrap();
        assert!(attrs.contains_key("NAME"));
        assert!(attrs.contains_key("VALUE"));
        assert!(attrs.contains_key("BANDWIDTH"));
    }

    #[test]
    fn test_parse_quoted_string() {
        assert_eq!(parse_quoted_string("\"hello\"").unwrap(), "hello");
        assert!(parse_quoted_string("hello").is_err());
        assert!(parse_quoted_string("\"hello\n\"").is_err());
    }
}
