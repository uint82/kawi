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
    if encoding == &PositionEncodingKind::UTF8 {
        line.len()
    } else if encoding == &PositionEncodingKind::UTF32 {
        line.chars().count()
    } else {
        line.encode_utf16().count()
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
}
