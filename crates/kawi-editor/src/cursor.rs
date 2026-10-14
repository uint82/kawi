pub struct Cursor {
    row: usize,
    col: usize,
    desired: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            row: 0,
            col: 0,
            desired: 0,
        }
    }

    pub fn row(&self) -> usize {
        self.row
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn desired(&self) -> usize {
        self.desired
    }

    pub fn set_position(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
        self.desired = col;
    }

    pub fn set_row(&mut self, row: usize) {
        self.row = row;
    }

    pub fn set_col(&mut self, col: usize) {
        self.col = col;
    }

    pub fn set_desired(&mut self, desired: usize) {
        self.desired = desired;
    }

    pub fn move_up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.row = self.row.saturating_add(1);
    }

    pub fn move_left(&mut self) {
        self.col = self.col.saturating_sub(1);
        self.desired = self.col;
    }

    pub fn move_right(&mut self) {
        self.col = self.col.saturating_add(1);
        self.desired = self.col;
    }

    pub fn advance_col(&mut self) {
        self.col = self.col.saturating_add(1);
        self.desired = self.col;
    }

    pub fn move_to_line_start(&mut self, row: usize) {
        self.row = row;
        self.col = 0;
        self.desired = 0;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_updates_desired() {
        let mut cursor = Cursor::new();
        cursor.move_right();
        cursor.move_right();
        assert_eq!(cursor.desired(), 2);
        cursor.move_left();
        assert_eq!(cursor.desired(), 1);
    }

    #[test]
    fn vertical_keeps_desired() {
        let mut cursor = Cursor::new();
        cursor.move_right();
        cursor.move_right();
        cursor.move_down();
        assert_eq!(cursor.desired(), 2);
    }

    #[test]
    fn place_remembers_col() {
        let mut cursor = Cursor::new();
        cursor.set_position(3, 7);
        assert_eq!(cursor.desired(), 7);
    }
}
