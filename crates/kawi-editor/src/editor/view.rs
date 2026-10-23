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
