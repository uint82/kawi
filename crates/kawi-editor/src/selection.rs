#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    anchor_row: usize,
    anchor_col: usize,
    head_row: usize,
    head_col: usize,
}

impl Selection {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            anchor_row: row,
            anchor_col: col,
            head_row: row,
            head_col: col,
        }
    }

    pub fn from_ends(anchor: (usize, usize), head: (usize, usize)) -> Self {
        Self {
            anchor_row: anchor.0,
            anchor_col: anchor.1,
            head_row: head.0,
            head_col: head.1,
        }
    }

    pub fn set_head(&mut self, row: usize, col: usize) {
        self.head_row = row;
        self.head_col = col;
    }

    pub fn anchor(&self) -> (usize, usize) {
        (self.anchor_row, self.anchor_col)
    }

    pub fn head(&self) -> (usize, usize) {
        (self.head_row, self.head_col)
    }

    pub fn swap_ends(&mut self) {
        std::mem::swap(&mut self.anchor_row, &mut self.head_row);
        std::mem::swap(&mut self.anchor_col, &mut self.head_col);
    }

    pub fn start(&self) -> (usize, usize) {
        self.ordered().0
    }

    pub fn end(&self) -> (usize, usize) {
        self.ordered().1
    }

    fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        let anchor = (self.anchor_row, self.anchor_col);
        let head = (self.head_row, self.head_col);
        if anchor <= head {
            (anchor, head)
        } else {
            (head, anchor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_point_has_same_ends() {
        let selection = Selection::new(1, 2);
        assert_eq!(selection.start(), (1, 2));
        assert_eq!(selection.end(), (1, 2));
    }

    #[test]
    fn forward_head_orders() {
        let mut selection = Selection::new(0, 0);
        selection.set_head(0, 3);
        assert_eq!(selection.start(), (0, 0));
        assert_eq!(selection.end(), (0, 3));
    }

    #[test]
    fn backward_head_orders() {
        let mut selection = Selection::new(1, 4);
        selection.set_head(0, 1);
        assert_eq!(selection.start(), (0, 1));
        assert_eq!(selection.end(), (1, 4));
    }

    #[test]
    fn multiline_orders_by_row() {
        let mut selection = Selection::new(0, 5);
        selection.set_head(2, 1);
        assert_eq!(selection.start(), (0, 5));
        assert_eq!(selection.end(), (2, 1));
    }
}
