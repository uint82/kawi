use super::*;

impl DebugUi {
    pub(super) fn push_output(&mut self, category: &str, output: &str) {
        for line in output.lines() {
            if self.output.len() == MAX_OUTPUT_LINES {
                self.output.pop_front();
            }
            self.output.push_back(format!("[{category}] {line}"));
        }
    }
}
