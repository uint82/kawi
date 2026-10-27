use super::Rect;
use crate::text::char_width;

pub struct KeyMenu<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub breadcrumb: &'a str,
    pub hint: Option<&'a str>,
    pub rows: &'a [KeyRow<'a>],
    pub columns: &'a [KeyColumn],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyRow<'a> {
    pub key: char,
    pub aliases: &'a [char],
    pub description: &'a str,
}

impl KeyRow<'_> {
    pub fn key_label(&self) -> String {
        let mut label = self.key.to_string();
        for alias in self.aliases {
            label.push('/');
            label.push(*alias);
        }
        label
    }

    pub fn key_width(&self) -> usize {
        self.aliases
            .iter()
            .fold(char_width(self.key), |width, alias| {
                width.saturating_add(char_width(*alias)).saturating_add(1)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyColumn {
    pub start: usize,
    pub len: usize,
    pub offset: usize,
    pub width: usize,
    pub key_width: usize,
    pub description_offset: usize,
}
