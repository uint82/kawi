use crate::responder::TerminalQuery;

use super::TerminalSession;

impl TerminalSession {
    pub(super) fn respond_to(&mut self, queries: Vec<TerminalQuery>) {
        for query in queries {
            if let Some(response) = self.response(query) {
                let _ = self.pty.write(&response);
            }
        }
    }

    fn response(&self, query: TerminalQuery) -> Option<Vec<u8>> {
        match query {
            TerminalQuery::BackgroundColor => Some(b"\x1b]11;rgb:1414/1616/1b1b\x1b\\".to_vec()),
            TerminalQuery::ForegroundColor => Some(b"\x1b]10;rgb:e0e0/e2e2/eaea\x1b\\".to_vec()),
            TerminalQuery::CursorColor => Some(b"\x1b]12;rgb:e0e0/e2e2/eaea\x1b\\".to_vec()),
            TerminalQuery::CursorPosition => {
                let cursor = self.screen.cursor();
                Some(
                    format!(
                        "\x1b[{};{}R",
                        cursor.row.saturating_add(1),
                        cursor.col.saturating_add(1)
                    )
                    .into_bytes(),
                )
            }
            TerminalQuery::DeviceAttributes => Some(b"\x1b[?1;2c".to_vec()),
            TerminalQuery::SecondaryDeviceAttributes => Some(b"\x1b[>0;0;0c".to_vec()),
            TerminalQuery::StatusReport => Some(b"\x1b[0n".to_vec()),
            TerminalQuery::KittyKeyboard => Some(b"\x1b[?0u".to_vec()),
            TerminalQuery::XtGetTcap(key) => {
                let mut response = b"\x1bP0+r".to_vec();
                response.extend_from_slice(&key);
                response.extend_from_slice(b"\x1b\\");
                Some(response)
            }
            TerminalQuery::DecMode(mode) => {
                Some(format!("\x1b[?{};{}$y", mode, self.mode_state(mode)).into_bytes())
            }
        }
    }

    fn mode_state(&self, mode: u16) -> u8 {
        let set = match mode {
            1 => self.screen.application_cursor(),
            25 => self.screen.cursor().visible,
            1049 => self.screen.alternate_screen(),
            2004 => self.screen.bracketed_paste(),
            _ => return 0,
        };
        if set {
            1
        } else {
            2
        }
    }
}
