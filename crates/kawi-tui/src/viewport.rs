pub struct Viewport {
    top: usize,
    height: usize,
    width: usize,
}

impl Viewport {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            top: 0,
            height: normalize_size(height),
            width: normalize_size(width),
        }
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = normalize_size(width);
        self.height = normalize_size(height);
    }

    pub fn ensure_visible(&mut self, cursor_row: usize) {
        self.scroll_down_if_needed(cursor_row);
        self.scroll_up_if_needed(cursor_row);
    }

    pub fn scroll_lines(&mut self, delta: isize, line_count: usize) {
        let shifted = self.top.saturating_add_signed(delta);
        self.top = shifted.min(max_top(line_count, self.height));
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn visible_end(&self) -> usize {
        self.top.saturating_add(self.height)
    }

    pub fn screen_row(&self, cursor_row: usize) -> usize {
        cursor_row.saturating_sub(self.top)
    }

    fn scroll_down_if_needed(&mut self, cursor_row: usize) {
        if cursor_row >= self.visible_end() {
            self.top = cursor_row.saturating_sub(self.height.saturating_sub(1));
        }
    }

    fn scroll_up_if_needed(&mut self, cursor_row: usize) {
        if cursor_row < self.top {
            self.top = cursor_row;
        }
    }
}

fn normalize_size(v: usize) -> usize {
    v.max(1)
}

fn max_top(line_count: usize, height: usize) -> usize {
    line_count.saturating_sub(height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_top() {
        let v = Viewport::new(10, 80);
        assert_eq!(v.top(), 0);
    }

    #[test]
    fn scrolls_down_when_cursor_below() {
        let mut v = Viewport::new(10, 80);
        v.ensure_visible(10);
        assert_eq!(v.top(), 1);
    }

    #[test]
    fn scrolls_up_when_cursor_above() {
        let mut v = Viewport::new(10, 80);
        v.ensure_visible(15);
        v.ensure_visible(2);
        assert_eq!(v.top(), 2);
    }

    #[test]
    fn stays_when_cursor_visible() {
        let mut v = Viewport::new(10, 80);
        v.ensure_visible(5);
        assert_eq!(v.top(), 0);
    }

    #[test]
    fn screen_row_subtracts_top() {
        let mut v = Viewport::new(10, 80);
        v.ensure_visible(12);
        assert_eq!(v.screen_row(12), 12 - v.top());
    }

    #[test]
    fn wheel_down_shifts_top() {
        let mut v = Viewport::new(10, 80);
        v.scroll_lines(3, 100);
        assert_eq!(v.top(), 3);
    }

    #[test]
    fn wheel_up_saturates_at_zero() {
        let mut v = Viewport::new(10, 80);
        v.scroll_lines(5, 100);
        v.scroll_lines(-9, 100);
        assert_eq!(v.top(), 0);
    }

    #[test]
    fn wheel_down_stops_at_bottom() {
        let mut v = Viewport::new(10, 80);
        v.scroll_lines(500, 100);
        assert_eq!(v.top(), 90);
    }

    #[test]
    fn short_file_never_scrolls() {
        let mut v = Viewport::new(10, 80);
        v.scroll_lines(3, 4);
        assert_eq!(v.top(), 0);
    }
}
