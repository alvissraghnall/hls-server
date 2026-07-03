use std::{fmt::Display, str::FromStr};

use crate::{
    attribute_list::{parse_attribute_list},
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
                Ok(SharedTag::Version(v))
            } else {
                Err(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))
            }
        }
        "#EXT-X-INDEPENDENT-SEGMENTS" => Ok(SharedTag::IndependentSegments),
        s if s.starts_with("#EXT-X-START:") => {
            let s = s
                .strip_prefix("#EXT-X-START:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
                )))?;
            let attrs = parse_attribute_list(s)?;

            let time_offset = attrs
                .get("TIME-OFFSET")
                .and_then(super::attribute_list::AttributeValue::as_signed_decimal_floating_point)
                .ok_or(ParseError::InvalidAttributeValue {
                    attribute: "TIME-OFFSET".into(),
                    value: "NONE".into(),
                    expected: "a valid signed decimal floating point number",
                })?;
            let precise = attrs
                .get("PRECISE")
                .and_then(|v| v.as_enumerated_string())
                .is_some_and(|v| v == "YES");
            Ok(SharedTag::Start {
                precise,
                time_offset,
            })
        }
        s if s.starts_with("#EXT-X-DEFINE:") => {
            let attrs = s
                .strip_prefix("#EXT-X-DEFINE:")
                .ok_or(ParseError::InvalidLine(format!(
                    "{line} is not valid according to HLS spec."
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

impl Display for SharedTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedTag::Version(v) => write!(f, "#EXT-X-VERSION:{v}"),
            SharedTag::IndependentSegments => write!(f, "#EXT-X-INDEPENDENT-SEGMENTS"),
            SharedTag::Start {
                precise,
                time_offset,
            } => write!(
                f,
                "#EXT-X-START:PRECISE={precise},TIME-OFFSET={time_offset}"
            ),
            SharedTag::Variable(var) => write!(f, "#EXT-X-DEFINE:{var}"),
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
            SharedTag::Variable(_play_list_variable_definition) => todo!(),
            SharedTag::IndependentSegments => todo!(),
            SharedTag::Start {
                precise: _,
                time_offset: _,
            } => todo!(),
        }

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
