use std::borrow::Cow;
use std::io::{self, Write};

const LF_BYTE: u8 = b'\n';
const CRLF: &[u8] = b"\r\n";
const LF: &[u8] = b"\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn detect(text: &str) -> Self {
        let newlines = count_byte(text, LF_BYTE);
        let crlf = text.match_indices("\r\n").count();
        if crlf > newlines.saturating_sub(crlf) {
            Self::Crlf
        } else {
            Self::Lf
        }
    }

    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => LF,
            Self::Crlf => CRLF,
        }
    }

    pub fn write(self, out: &mut impl Write, text: &str) -> io::Result<()> {
        match self {
            Self::Lf => out.write_all(text.as_bytes()),
            Self::Crlf => write_crlf(out, text),
        }
    }

    pub fn encoded_len(self, text: &str) -> usize {
        match self {
            Self::Lf => text.len(),
            Self::Crlf => text.len().saturating_add(count_byte(text, LF_BYTE)),
        }
    }
}

pub fn normalize(text: &str) -> Cow<'_, str> {
    if !text.contains("\r\n") {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.replace("\r\n", "\n"))
}

fn write_crlf(out: &mut impl Write, text: &str) -> io::Result<()> {
    let mut start = 0;
    for (index, _) in text.match_indices('\n') {
        out.write_all(&text.as_bytes()[start..index])?;
        out.write_all(CRLF)?;
        start = index.saturating_add(1);
    }
    out.write_all(&text.as_bytes()[start..])
}

fn count_byte(text: &str, byte: u8) -> usize {
    text.bytes().filter(|current| *current == byte).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_line_endings() {
        assert_eq!(LineEnding::detect(""), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\nb\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a\r\nb\r\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("a\r\nb\nc\n"), LineEnding::Lf);
    }

    #[test]
    fn normalizes_crlf_to_lf() {
        assert_eq!(normalize("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(normalize("a\nb\n"), "a\nb\n");
        assert_eq!(normalize("a\rb"), "a\rb");
    }

    #[test]
    fn encodes_crlf() {
        let mut text = Vec::new();
        LineEnding::Crlf.write(&mut text, "a\nb").expect("write");
        assert_eq!(text, b"a\r\nb");
    }

    #[test]
    fn measures_encoded_length() {
        assert_eq!(LineEnding::Lf.encoded_len("a\nb"), 3);
        assert_eq!(LineEnding::Crlf.encoded_len("a\nb"), 4);
        assert_eq!(LineEnding::Crlf.encoded_len("a\rb"), 3);
    }
}
