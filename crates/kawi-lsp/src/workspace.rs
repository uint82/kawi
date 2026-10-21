use anyhow::{Context, Result};
use lsp_types::{Position, PositionEncodingKind, Range, Uri};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn file_uri(path: &Path) -> Result<Uri> {
    let path = absolute(path);
    let encoded = encode_path(&path.to_string_lossy());
    let uri = format!("file://{encoded}");
    Uri::from_str(&uri).with_context(|| format!("build file uri for {}", path.display()))
}

pub fn whole_document_range(text: &str, encoding: &PositionEncodingKind) -> Range {
    let line = text.bytes().filter(|byte| *byte == b'\n').count();
    let last_line = text.rsplit('\n').next().unwrap_or(text);
    let character = encoded_len(last_line, encoding);
    Range {
        start: Position::new(0, 0),
        end: Position::new(
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(character).unwrap_or(u32::MAX),
        ),
    }
}

pub(crate) fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn path_from_uri(uri: &Uri) -> Option<PathBuf> {
    let encoded = uri.as_str().strip_prefix("file://")?;
    let decoded = decode_path(encoded);
    Some(PathBuf::from(decoded))
}

pub(crate) fn char_index(text: &str, character: u32, encoding: &PositionEncodingKind) -> u32 {
    let mut units = 0u32;
    for (index, value) in text.chars().enumerate() {
        if units >= character {
            return u32::try_from(index).unwrap_or(u32::MAX);
        }
        units = units.saturating_add(encoded_char_len(value, encoding));
    }
    u32::try_from(text.chars().count()).unwrap_or(u32::MAX)
}

pub(crate) fn root_for(path: &Path, markers: &[String]) -> PathBuf {
    let path = absolute(path);
    let directory = path.parent().unwrap_or_else(|| Path::new(""));
    for ancestor in directory.ancestors() {
        if markers.iter().any(|marker| ancestor.join(marker).exists()) {
            return ancestor.to_path_buf();
        }
    }
    directory.to_path_buf()
}

fn encoded_len(line: &str, encoding: &PositionEncodingKind) -> usize {
    line.chars()
        .map(|value| usize::try_from(encoded_char_len(value, encoding)).unwrap_or(usize::MAX))
        .sum()
}

fn encoded_char_len(value: char, encoding: &PositionEncodingKind) -> u32 {
    if encoding == &PositionEncodingKind::UTF8 {
        u32::try_from(value.len_utf8()).unwrap_or(u32::MAX)
    } else if encoding == &PositionEncodingKind::UTF32 {
        1
    } else {
        u32::try_from(value.len_utf16()).unwrap_or(u32::MAX)
    }
}

fn decode_path(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[(byte >> 4) as usize]));
            encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
    encoded
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn builds_escaped_file_uris() {
        let uri = file_uri(Path::new("/tmp/a b#c.rs")).expect("uri");
        assert!(uri.as_str().starts_with("file:///tmp/a%20b%23c.rs"));
    }

    #[test]
    fn encodes_non_ascii_paths() {
        let uri = file_uri(Path::new("/tmp/naïve.rs")).expect("uri");
        assert!(uri.as_str().starts_with("file:///tmp/na%C3%AFve.rs"));
    }

    #[test]
    fn finds_the_nearest_root_marker() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_root_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        let markers = vec!["Cargo.toml".to_string()];
        assert_eq!(root_for(&root.join("src/main.rs"), &markers), root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn falls_back_to_the_parent_directory() {
        let path = Path::new("/tmp/kawi-lsp-no-marker/src/main.rs");
        assert_eq!(
            root_for(path, &["definitely-not-present".to_string()]),
            Path::new("/tmp/kawi-lsp-no-marker/src")
        );
    }

    #[test]
    fn computes_whole_document_ranges_in_utf16() {
        let range = whole_document_range("a\n𝄞", &PositionEncodingKind::UTF16);
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(1, 2));
    }

    #[test]
    fn computes_whole_document_ranges_with_trailing_newline() {
        let range = whole_document_range("a\n", &PositionEncodingKind::UTF16);
        assert_eq!(range.end, Position::new(1, 0));
    }

    #[test]
    fn computes_empty_document_ranges() {
        let range = whole_document_range("", &PositionEncodingKind::UTF16);
        assert_eq!(range.end, Position::new(0, 0));
    }

    #[test]
    fn converts_utf_units_to_character_indices() {
        assert_eq!(char_index("a𝄞b", 0, &PositionEncodingKind::UTF16), 0);
        assert_eq!(char_index("a𝄞b", 1, &PositionEncodingKind::UTF16), 1);
        assert_eq!(char_index("a𝄞b", 3, &PositionEncodingKind::UTF16), 2);
        assert_eq!(char_index("a𝄞b", 3, &PositionEncodingKind::UTF8), 2);
        assert_eq!(char_index("a𝄞b", 2, &PositionEncodingKind::UTF32), 2);
        assert_eq!(char_index("a𝄞b", 99, &PositionEncodingKind::UTF16), 3);
    }

    #[test]
    fn decodes_file_uris() {
        let uri = file_uri(Path::new("/tmp/a b#c.rs")).expect("uri");
        assert_eq!(path_from_uri(&uri), Some(PathBuf::from("/tmp/a b#c.rs")));
        let unicode = file_uri(Path::new("/tmp/naïve.rs")).expect("unicode uri");
        assert_eq!(
            path_from_uri(&unicode),
            Some(PathBuf::from("/tmp/naïve.rs"))
        );
        assert!(path_from_uri(&Uri::from_str("https://example.com/x").unwrap()).is_none());
    }
}
