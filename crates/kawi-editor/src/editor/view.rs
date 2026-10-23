use super::Editor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ViewState {
    pub row: usize,
    pub col: usize,
    pub desired: usize,
    pub top: usize,
    pub leftcol: usize,
}

impl Editor {
    pub fn view_state(&self) -> ViewState {
        ViewState {
            row: self.cursor.row(),
            col: self.cursor.col(),
            desired: self.cursor.desired(),
            top: self.viewport.top(),
            leftcol: self.viewport.leftcol(),
        }
    }

    pub fn restore_view_state(&mut self, state: ViewState) {
        self.cursor.set_row(state.row);
        self.cursor.set_col(state.col);
        self.cursor.set_desired(state.desired);
        self.clamp_row();
        self.clamp_col();
        self.viewport.place_top(state.top, self.buffer.line_count());
        self.viewport.set_leftcol(state.leftcol);
    }

    pub fn clamp_view_state(&self, buffer: usize, state: ViewState) -> ViewState {
        let rows = self.buffer_line_count(buffer).unwrap_or(1).max(1);
        let row = state.row.min(rows.saturating_sub(1));
        let width = self.buffer_line_len(buffer, row).unwrap_or(0);
        let max_col = width.saturating_sub(1);
        let max_leftcol = self
            .buffer_virtcol(buffer, row, width)
            .unwrap_or(0)
            .saturating_sub(1);
        ViewState {
            row,
            col: state.col.min(max_col),
            desired: state.desired.min(max_col),
            top: state.top.min(rows.saturating_sub(1)),
            leftcol: state.leftcol.min(max_leftcol),
        }
    }
}

#[cfg(test)]
mod tests {
    use kawi_buffer::Buffer;

    use super::*;

    fn editor() -> Editor {
        let mut editor = Editor::new(Buffer::from_string("alpha\nbeta\ngamma\n"));
        editor.set_view_size(20, 2);
        editor
    }

    #[test]
    fn captures_and_restores_the_view() {
        let mut editor = editor();
        editor.restore_view_state(ViewState {
            row: 2,
            col: 3,
            desired: 3,
            top: 1,
            leftcol: 2,
        });
        assert_eq!(
            editor.view_state(),
            ViewState {
                row: 2,
                col: 3,
                desired: 3,
                top: 1,
                leftcol: 2,
            }
        );
    }

    #[test]
    fn clamps_a_view_to_a_changed_buffer() {
        let editor = editor();
        let clamped = editor.clamp_view_state(
            editor.buffer_number(),
            ViewState {
                row: 20,
                col: 20,
                desired: 20,
                top: 20,
                leftcol: 20,
            },
        );
        assert_eq!(
            clamped,
            ViewState {
                row: 2,
                col: 4,
                desired: 4,
                top: 2,
                leftcol: 4,
            }
        );
    }

    #[test]
    fn clamps_out_of_range_positions() {
        let mut editor = editor();
        editor.restore_view_state(ViewState {
            row: 20,
            col: 20,
            desired: 20,
            top: 20,
            leftcol: 20,
        });
        assert_eq!(editor.row(), 2);
        assert_eq!(editor.col(), 4);
        assert_eq!(editor.view_top(), 2);
        assert_eq!(editor.view_leftcol(), 20);
    }
}
