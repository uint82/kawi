use super::{Cell, Style, Surface};

impl Surface {
    pub(crate) fn row(&self, y: usize) -> Option<&[Cell]> {
        if y >= self.height {
            return None;
        }
        let start = y.saturating_mul(self.width);
        let end = start.saturating_add(self.width);
        self.cells.get(start..end)
    }

    pub(crate) fn row_is_width_stable(&self, y: usize) -> bool {
        self.row(y)
            .is_some_and(|cells| cells.iter().all(Cell::is_width_stable))
    }

    pub(crate) fn base_style(&self) -> Style {
        self.base
    }
}
