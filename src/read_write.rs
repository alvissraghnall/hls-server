use std::{fs, path::Path};

use crate::{error::PlaylistReadError};

pub(crate) fn read_from_file(path: impl AsRef<Path>) -> Result<String, PlaylistReadError> {
    let mut bytes = fs::read(path)?;

    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = bytes[3..].to_vec();
    }

    let content = String::from_utf8(bytes)?;

    validate_playlist_text(&content)?;

    Ok(content)
}

fn validate_playlist_text(content: &str) -> Result<(), PlaylistReadError> {
    for ch in content.chars() {
        match ch {
            '\n' | '\r' => continue,

            '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}' => {
                return Err(PlaylistReadError::InvalidControlCharacter(ch));
            }

            _ => {}
        }
    }

    Ok(())
}
