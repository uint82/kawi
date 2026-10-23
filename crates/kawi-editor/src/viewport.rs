pub const FALLBACK_WIDTH: usize = 80;
pub const FALLBACK_HEIGHT: usize = 24;

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    top: usize,
    leftcol: usize,
    height: usize,
    width: usize,
}

impl Viewport {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            top: 0,
            leftcol: 0,
            height: normalize_size(height),
            width: normalize_size(width),
        }
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = normalize_size(width);
        self.height = normalize_size(height);
    }

    pub fn scroll_lines(&mut self, delta: isize, line_count: usize) {
        let shifted = self.top.saturating_add_signed(delta);
        self.top = shifted.min(max_top(line_count, self.height));
    }

    pub fn place_top(&mut self, top: usize, line_count: usize) {
        self.top = top.min(line_count.saturating_sub(1));
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn leftcol(&self) -> usize {
        self.leftcol
    }

    pub fn set_leftcol(&mut self, leftcol: usize) {
        self.leftcol = leftcol;
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
    fn leftcol_starts_at_zero() {
        let v = Viewport::new(10, 80);
        assert_eq!(v.leftcol(), 0);
    }

    #[test]
    fn leftcol_can_be_set() {
        let mut v = Viewport::new(10, 80);
        v.set_leftcol(12);
        assert_eq!(v.leftcol(), 12);
    }

    #[test]
    fn screen_row_subtracts_top() {
        let mut v = Viewport::new(10, 80);
        v.place_top(3, 100);
        assert_eq!(v.screen_row(12), 9);
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

    #[test]
    fn place_top_past_max_sticks() {
        let mut v = Viewport::new(10, 80);
        v.place_top(95, 100);
        assert_eq!(v.top(), 95);
    }

    #[test]
    fn place_top_clamps_to_last_line() {
        let mut v = Viewport::new(10, 80);
        v.place_top(500, 100);
        assert_eq!(v.top(), 99);
    }
}
