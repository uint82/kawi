pub struct Cursor {
    row: usize,
    col: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self { row: 0, col: 0 }
    }

    pub fn row(&self) -> usize {
        self.row
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn set_position(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    pub fn set_row(&mut self, row: usize) {
        self.row = row;
    }

    pub fn set_col(&mut self, col: usize) {
        self.col = col;
    }

    pub fn move_up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.row = self.row.saturating_add(1);
    }

    pub fn move_left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.col = self.col.saturating_add(1);
    }

    pub fn advance_col(&mut self) {
        self.col = self.col.saturating_add(1);
    }

    pub fn move_to_line_start(&mut self, row: usize) {
        self.row = row;
        self.col = 0;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}
