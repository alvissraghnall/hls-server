use crate::{
    attribute_list::{AttributeList, AttributeValue, parse_attribute_list},
    error::ParseError,
    playlist::{PlayListVariableDefinition, SharedTag},
};

pub(crate) fn parse_shared_tag(line: &str) -> Result<SharedTag, ParseError> {
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
                .ok_or(ParseError::InvalidAttributeValue(
                    "TIME-OFFSET is REQUIRED".to_string(),
                ))?;
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
        _ => Err(ParseError::UnknownTag(format!(
            "{} is not valid according to HLS spec.",
            line
        ))),
    }
}

fn parse_variable_definition(
    attrs: &AttributeList,
) -> Result<PlayListVariableDefinition, ParseError> {
    if let (Some(name), Some(value)) = (attrs.get("NAME"), attrs.get("VALUE")) {
        return Ok(PlayListVariableDefinition::NameValue {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    if let Some(import) = attrs.get("IMPORT") {
        return Ok(PlayListVariableDefinition::Import {
            name: import.to_string(),
        });
    }

    if let Some(query_param) = attrs.get("QUERY_PARAM") {
        return Ok(PlayListVariableDefinition::QueryParam {
            name: query_param.to_string(),
            value: String::new(),
        });
    }

    Err(ParseError::UnknownTag(format!("{:?}", attrs.keys())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{attribute_list::parse_quoted_string, playlist::PlayListVariableDefinition};

    #[test]
    fn test_parse_version() {
        let tag = parse_shared_tag("#EXT-X-VERSION:3").unwrap();
        assert_eq!(tag, SharedTag::Version(3));

        assert!(parse_shared_tag("#EXT-X-VERSION:abc").is_err());
    }

    #[test]
    fn test_parse_independent_segments() {
        let tag = parse_shared_tag("#EXT-X-INDEPENDENT-SEGMENTS").unwrap();
        assert_eq!(tag, SharedTag::IndependentSegments);
    }

    #[test]
    fn test_parse_start() {
        let tag = parse_shared_tag("#EXT-X-START:TIME-OFFSET=10.5,PRECISE=YES").unwrap();
        assert_eq!(
            tag,
            SharedTag::Start {
                precise: true,
                time_offset: 10.5
            }
        );

        let tag = parse_shared_tag("#EXT-X-START:TIME-OFFSET=-2.0").unwrap();
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
        let tag = parse_shared_tag("#EXT-X-DEFINE:NAME=\"VAR\",VALUE=\"val\"").unwrap();
        if let SharedTag::Variable(PlayListVariableDefinition::NameValue { name, value }) = tag {
            assert_eq!(name, "VAR");
            assert_eq!(value, "val");
        } else {
            panic!("Expected NameValue");
        }
    }

    #[test]
    fn test_parse_define_import() {
        let tag = parse_shared_tag("#EXT-X-DEFINE:IMPORT=\"VAR\"").unwrap();
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

// fn build_playlist_def(mut map: AttributeList) -> Result<PlayListVariableDefinition, ParseError> {
//     let has_import = map.contains_key("IMPORT");
//     let has_query = map.contains_key("QUERY");
//     let has_namevalue = map.contains_key("NAME") || map.contains_key("VALUE");

//     let count = has_import as u8 + has_query as u8 + has_namevalue as u8;

//     match count {
//         0 => return Err(ParseError::NoAttribute),
//         2.. => return Err(ParseError::TooManyAttributes),
//         _ => {}
//     }

//     if has_import {
//         let raw = map
//             .remove("IMPORT")
//             .ok_or(ParseError::UnknownAttribute("IMPORT".to_string()))?;

//         return Ok(PlayListVariableDefinition::Import {
//             name: extract_quoted_string(raw)?,
//         });
//     }

//     if has_query {
//         let raw = map
//             .remove("QUERY")
//             .ok_or(ParseError::UnknownAttribute("QUERY".to_string()))?;

//         return Ok(PlayListVariableDefinition::QueryParam {
//             name: extract_quoted_string(raw)?,
//         });
//     }

//     if has_namevalue {
//         let raw_name = map
//             .remove("NAME")
//             .ok_or(ParseError::UnknownAttribute("NAME".to_string()))?;
//         let raw_value = map
//             .remove("VALUE")
//             .ok_or(ParseError::UnknownAttribute("VALUE".to_string()))?;

//         return Ok(PlayListVariableDefinition::NameValue {
//             name: extract_quoted_string(raw_name)?,
//             value: extract_quoted_string(raw_value)?,
//         });
//     }

//     Err(ParseError::NoAttribute)
// }
