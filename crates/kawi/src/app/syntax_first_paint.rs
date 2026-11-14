use std::time::{Duration, Instant};

use kawi_syntax::SyntaxKey;

use super::App;

// Covers most source files without putting a large parse on the critical path.
const FIRST_PAINT_MAX_BYTES: usize = 128 * 1024;
// Half a frame: enough for a warm parse, short enough that a cold one is barely late.
const FIRST_PAINT_TIMEOUT: Duration = Duration::from_millis(8);

impl App {
    pub(crate) fn wait_for_first_paint(&mut self) {
        let Some(key) = self.syntax_key else {
            return;
        };
        let deadline = Instant::now() + FIRST_PAINT_TIMEOUT;
        self.wait_for_syntax_until(key, deadline);
    }

    fn wait_for_syntax_until(&mut self, key: SyntaxKey, deadline: Instant) -> bool {
        loop {
            if self.syntax_spans_cover(&key) {
                return true;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            let Some(result) = self
                .syntax
                .as_ref()
                .and_then(|service| service.recv_timeout(remaining))
            else {
                return false;
            };
            self.apply_syntax_result(result);
        }
    }
}

pub(super) fn fits_first_paint(text_bytes: usize) -> bool {
    text_bytes <= FIRST_PAINT_MAX_BYTES
}

#[cfg(test)]
mod tests;
