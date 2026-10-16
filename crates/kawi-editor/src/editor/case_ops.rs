use super::Editor;

impl Editor {
    pub(super) fn toggle_case(&mut self) {
        self.shift_selection_case(toggle_ascii_case);
    }

    pub(super) fn lowercase(&mut self) {
        self.shift_selection_case(lower_ascii_case);
    }

    pub(super) fn uppercase(&mut self) {
        self.shift_selection_case(upper_ascii_case);
    }

    fn shift_selection_case(&mut self, shift: fn(char) -> char) {
        let (start, end) = self.operator_range();
        self.record_boundary();
        self.buffer
            .shift_case_range(start.0, start.1, end.0, end.1, shift);
        self.collapse_to(start.0, start.1);
        self.cursor.set_position(start.0, start.1);
    }
}

// TODO(case): match vim towupper for non-ASCII text.
fn toggle_ascii_case(ch: char) -> char {
    if ch.is_ascii_lowercase() {
        ch.to_ascii_uppercase()
    } else if ch.is_ascii_uppercase() {
        ch.to_ascii_lowercase()
    } else {
        ch
    }
}

fn lower_ascii_case(ch: char) -> char {
    ch.to_ascii_lowercase()
}

fn upper_ascii_case(ch: char) -> char {
    ch.to_ascii_uppercase()
}
