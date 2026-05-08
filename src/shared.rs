use crate::playlist::SharedTag;

pub(crate) fn parse_shared_tag(line: &str) -> SharedTag {
    let mut tag = SharedTag::default();

    match line {
        s if s.starts_with("#EXT-X-VERSION:") => {
            let version = s
                .strip_prefix("#EXT-X-VERSION:")
                .and_then(|v| v.parse::<u8>().ok());

            if let Some(v) = version {
                tag = SharedTag::Version(v);
            }
        }
        "#EXT-X-INDEPENDENT-SEGMENTS" => tag = SharedTag::IndependentSegments,
        "#EXT-X-START:PRECISE=YES" => {
            tag = SharedTag::Start {
                precise: true,
                time_offset: 0.0,
            }
        }
        "#EXT-X-START:TIME-OFFSET=10.5" => {
            tag = SharedTag::Start {
                precise: false,
                time_offset: 10.5,
            }
        }

        _ => {}
    }
    tag
}
