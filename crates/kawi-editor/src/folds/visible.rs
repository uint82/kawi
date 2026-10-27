use super::FoldState;

impl FoldState {
    pub fn is_hidden(&self, row: usize) -> bool {
        self.folds()
            .iter()
            .any(|fold| self.is_closed(fold.start) && fold.start < row && row <= fold.end)
    }

    pub fn visible_rows(&self, top: usize, height: usize, line_count: usize) -> Vec<usize> {
        let mut rows = Vec::with_capacity(height);
        if line_count == 0 {
            return rows;
        }
        let mut row = self.visible_top(top, line_count);
        for _ in 0..height {
            if row >= line_count {
                break;
            }
            rows.push(row);
            if row.saturating_add(1) >= line_count {
                break;
            }
            row = self.next_visible(row.saturating_add(1), line_count);
        }
        rows
    }

    pub fn visible_line_count(&self, line_count: usize) -> usize {
        line_count.saturating_sub(self.hidden_before(line_count))
    }

    pub fn display_row(&self, row: usize, line_count: usize) -> usize {
        row.min(line_count.saturating_sub(1))
            .saturating_sub(self.hidden_before(row))
    }

    pub fn buffer_row(&self, display: usize, line_count: usize) -> usize {
        if line_count == 0 {
            return 0;
        }
        let visible = self.visible_line_count(line_count);
        let mut row = display.min(visible.saturating_sub(1));
        for (start, end) in self.hidden_ranges() {
            if start > row {
                break;
            }
            row = row.saturating_add(end.saturating_sub(start).saturating_add(1));
        }
        row.min(line_count.saturating_sub(1))
    }

    pub fn visible_top(&self, row: usize, line_count: usize) -> usize {
        let next = self.next_visible(row, line_count);
        if next < line_count {
            next
        } else {
            self.prev_visible(row, line_count)
        }
    }

    pub fn next_visible(&self, row: usize, line_count: usize) -> usize {
        if line_count == 0 {
            return 0;
        }
        let mut row = row.min(line_count);
        while row < line_count && self.is_hidden(row) {
            row = row.saturating_add(1);
        }
        row
    }

    pub fn prev_visible(&self, row: usize, line_count: usize) -> usize {
        if line_count == 0 {
            return 0;
        }
        let mut row = row.min(line_count.saturating_sub(1));
        while row > 0 && self.is_hidden(row) {
            row = row.saturating_sub(1);
        }
        row
    }

    pub fn max_top(&self, line_count: usize, height: usize) -> usize {
        if line_count == 0 || self.visible_line_count(line_count) <= height {
            return 0;
        }
        let mut row = self.prev_visible(line_count.saturating_sub(1), line_count);
        for _ in 1..height {
            row = self.prev_visible(row.saturating_sub(1), line_count);
        }
        row
    }

    pub fn next_fold_start(&self, row: usize, line_count: usize) -> Option<usize> {
        self.folds()
            .iter()
            .map(|fold| fold.start)
            .find(|start| *start > row && !self.is_hidden(*start) && *start < line_count)
    }

    pub fn prev_fold_start(&self, row: usize) -> Option<usize> {
        self.folds()
            .iter()
            .rev()
            .map(|fold| fold.start)
            .find(|start| *start < row && !self.is_hidden(*start))
    }

    fn hidden_before(&self, row: usize) -> usize {
        let mut hidden: usize = 0;
        for (start, end) in self.hidden_ranges() {
            if start >= row {
                break;
            }
            hidden = hidden.saturating_add(row.min(end.saturating_add(1)).saturating_sub(start));
        }
        hidden
    }

    fn hidden_ranges(&self) -> Vec<(usize, usize)> {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for fold in self.folds() {
            if !self.is_closed(fold.start) || fold.start >= fold.end {
                continue;
            }
            let start = fold.start.saturating_add(1);
            let end = fold.end;
            if let Some(last) = ranges.last_mut() {
                if start <= last.1.saturating_add(1) {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            ranges.push((start, end));
        }
        ranges
    }
}
