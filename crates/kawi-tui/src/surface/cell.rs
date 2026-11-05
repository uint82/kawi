use super::Style;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    text: String,
    width: u8,
    style: Style,
}

impl Cell {
    pub fn blank(style: Style) -> Self {
        Self {
            text: String::from(" "),
            width: 1,
            style,
        }
    }

    pub fn continuation(style: Style) -> Self {
        Self {
            text: String::new(),
            width: 0,
            style,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn width(&self) -> usize {
        usize::from(self.width)
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn is_continuation(&self) -> bool {
        self.width == 0
    }

    pub(crate) fn is_width_stable(&self) -> bool {
        self.text.is_ascii()
    }

    pub fn reset(&mut self, style: Style) {
        self.text.clear();
        self.text.push(' ');
        self.width = 1;
        self.style = style;
    }

    pub fn set(&mut self, text: &str, width: u8, style: Style) {
        self.text.clear();
        self.text.push_str(text);
        self.width = width;
        self.style = style;
    }
}
