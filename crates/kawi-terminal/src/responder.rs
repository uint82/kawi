const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const MAX_SEQUENCE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalQuery {
    BackgroundColor,
    ForegroundColor,
    CursorColor,
    CursorPosition,
    DeviceAttributes,
    SecondaryDeviceAttributes,
    StatusReport,
    KittyKeyboard,
    XtGetTcap(Vec<u8>),
    DecMode(u16),
}

#[derive(Default)]
pub(crate) struct TerminalResponder {
    pending: Vec<u8>,
}

impl TerminalResponder {
    pub(crate) fn process(&mut self, bytes: &[u8]) -> Vec<TerminalQuery> {
        let mut queries = Vec::new();
        for &byte in bytes {
            self.push(byte, &mut queries);
        }
        queries
    }

    fn push(&mut self, byte: u8, queries: &mut Vec<TerminalQuery>) {
        if self.pending.is_empty() {
            if byte == ESC {
                self.pending.push(byte);
            }
            return;
        }
        if self.pending.len() == 1 && byte == ESC {
            self.pending[0] = ESC;
            return;
        }
        self.pending.push(byte);
        if !is_complete(&self.pending) {
            if self.pending.len() > MAX_SEQUENCE {
                self.pending.clear();
            }
            return;
        }
        if let Some(query) = parse_sequence(&self.pending) {
            queries.push(query);
        }
        self.pending.clear();
    }
}

fn is_complete(sequence: &[u8]) -> bool {
    if sequence.len() < 2 {
        return false;
    }
    match sequence[1] {
        b'[' => sequence.len() >= 3 && matches!(sequence[sequence.len() - 1], 0x40..=0x7e),
        b']' | b'P' | b'_' | b'^' | b'X' => {
            if sequence.last() == Some(&BEL) {
                return true;
            }
            sequence.len() >= 3
                && sequence[sequence.len() - 2] == ESC
                && sequence[sequence.len() - 1] == b'\\'
        }
        _ => true,
    }
}

fn parse_sequence(sequence: &[u8]) -> Option<TerminalQuery> {
    match sequence.get(1)? {
        b'[' => parse_csi(sequence),
        b']' => parse_osc(sequence),
        b'P' => parse_dcs(sequence),
        _ => None,
    }
}

fn parse_csi(sequence: &[u8]) -> Option<TerminalQuery> {
    let final_byte = *sequence.last()?;
    let params = &sequence[2..sequence.len().saturating_sub(1)];
    match final_byte {
        b'c' if params.is_empty() || params == b"0" => Some(TerminalQuery::DeviceAttributes),
        b'c' if params.starts_with(b">") => Some(TerminalQuery::SecondaryDeviceAttributes),
        b'n' => match params {
            b"5" => Some(TerminalQuery::StatusReport),
            b"6" | b"?6" => Some(TerminalQuery::CursorPosition),
            _ => None,
        },
        b'u' if params == b"?" => Some(TerminalQuery::KittyKeyboard),
        b'p' if params.starts_with(b"?") && params.ends_with(b"$") => {
            let digits = &params[1..params.len().saturating_sub(1)];
            parse_u16(digits).map(TerminalQuery::DecMode)
        }
        _ => None,
    }
}

fn parse_osc(sequence: &[u8]) -> Option<TerminalQuery> {
    let content = sequence_content(sequence)?;
    match content {
        b"10;?" => Some(TerminalQuery::ForegroundColor),
        b"11;?" => Some(TerminalQuery::BackgroundColor),
        b"12;?" => Some(TerminalQuery::CursorColor),
        _ => None,
    }
}

fn parse_dcs(sequence: &[u8]) -> Option<TerminalQuery> {
    let content = sequence_content(sequence)?;
    content
        .strip_prefix(b"+q")
        .map(|key| TerminalQuery::XtGetTcap(key.to_vec()))
}

fn sequence_content(sequence: &[u8]) -> Option<&[u8]> {
    let end = if sequence.last() == Some(&BEL) {
        sequence.len().checked_sub(1)?
    } else if sequence.ends_with(&[ESC, b'\\']) {
        sequence.len().checked_sub(2)?
    } else {
        return None;
    };
    sequence.get(2..end)
}

fn parse_u16(bytes: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(bytes).ok()?;
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queries(bytes: &[u8]) -> Vec<TerminalQuery> {
        TerminalResponder::default().process(bytes)
    }

    #[test]
    fn detects_cursor_position() {
        assert_eq!(queries(b"\x1b[6n"), vec![TerminalQuery::CursorPosition]);
        assert_eq!(queries(b"\x1b[?6n"), vec![TerminalQuery::CursorPosition]);
    }

    #[test]
    fn detects_colors() {
        assert_eq!(
            queries(b"\x1b]10;?\x1b\\"),
            vec![TerminalQuery::ForegroundColor]
        );
        assert_eq!(
            queries(b"\x1b]11;?\x1b\\"),
            vec![TerminalQuery::BackgroundColor]
        );
        assert_eq!(queries(b"\x1b]12;?\x07"), vec![TerminalQuery::CursorColor]);
    }

    #[test]
    fn detects_device_attributes_and_status() {
        assert_eq!(queries(b"\x1b[0c"), vec![TerminalQuery::DeviceAttributes]);
        assert_eq!(
            queries(b"\x1b[>0c"),
            vec![TerminalQuery::SecondaryDeviceAttributes]
        );
        assert_eq!(queries(b"\x1b[5n"), vec![TerminalQuery::StatusReport]);
    }

    #[test]
    fn detects_keyboard_queries() {
        assert_eq!(queries(b"\x1b[?u"), vec![TerminalQuery::KittyKeyboard]);
    }

    #[test]
    fn ignores_cursor_style_queries() {
        assert!(queries(b"\x1b[>0q").is_empty());
    }

    #[test]
    fn detects_capability_queries() {
        assert_eq!(
            queries(b"\x1bP+q696e646e\x1b\\"),
            vec![TerminalQuery::XtGetTcap(b"696e646e".to_vec())]
        );
    }

    #[test]
    fn detects_mode_queries() {
        assert_eq!(queries(b"\x1b[?2004$p"), vec![TerminalQuery::DecMode(2004)]);
    }

    #[test]
    fn handles_split_sequences() {
        let mut responder = TerminalResponder::default();
        assert!(responder.process(b"\x1b[6").is_empty());
        assert_eq!(responder.process(b"n"), vec![TerminalQuery::CursorPosition]);
    }

    #[test]
    fn ignores_screen_control_sequences() {
        assert!(queries(b"\x1b[?1049h\x1b[?1049l").is_empty());
        assert!(queries(b"plain text").is_empty());
    }
}
