use crate::{
    attribute_list::{AttributeList, AttributeValue},
    error::ParseError,
    playlist::{PlayListVariableDefinition, SharedTag},
};

pub(crate) fn parse_shared_tag(line: &str) -> Result<SharedTag, ParseError> {
    let mut tag = SharedTag::default();

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
            // of tje form:
            // #EXT-X-START:PRECISE=YES
            // or
            // #EXT-X-START:TIME-OFFSET=10.5
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
        // s if s.starts_with("EXT-X-DEFINE:") => {
        //     let name = s_split[1];
        //     let value = s_split[2];
        //     Ok(SharedTag::Variables(vec![PlayListVariableDefinition {
        //         name: name.to_string(),
        //         value: value.to_string(),
        //     }]))
        // }
        _ => Err(ParseError::UnknownTag(format!(
            "{} is not valid according to HLS spec.",
            line
        ))),
    }
}

fn parse_quoted_string(value: &str) -> Result<String, ParseError> {
    if !(value.starts_with('"') && value.ends_with('"')) {
        return Err(ParseError::ExpectedQuotedString);
    }

    let inner = &value[1..value.len() - 1];

    if inner.contains('"') || inner.contains('\n') || inner.contains('\r') {
        return Err(ParseError::InvalidQuotedString(inner.to_string()));
    }

    Ok(inner.to_string())
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
        });
    }

    Err(ParseError::UnknownTag(format!("{:?}", attrs.keys())))
}

fn parse_attribute_value(name: &str, value: &str) -> Result<AttributeValue, ParseError> {
    match name {
        "BANDWIDTH" => Ok(AttributeValue::DecimalInteger(value.parse()?)),

        "TIME-OFFSET" => Ok(AttributeValue::SignedDecimalFloatingPoint(value.parse()?)),

        "PRECISE" => Ok(AttributeValue::EnumeratedString(value.to_string())),

        "CODECS" => Ok(AttributeValue::QuotedString(parse_quoted_string(value)?)),

        _ => Err(ParseError::UnknownAttribute(name.into())),
    }
}

fn parse_attribute_list(s: &str) -> Result<AttributeList, ParseError> {
    let mut attrs = AttributeList::new();
    for (key, value) in s
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.split_once('='))
        .flatten()
    {
        let key = key.trim();
        let value = value.trim();
        attrs.insert(key.to_string(), parse_attribute_value(key, value)?);
    }
    Ok(attrs)
}
