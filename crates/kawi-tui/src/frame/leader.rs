use super::Rect;
use crate::text::char_width;

pub struct LeaderMenu<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub group: Option<char>,
    pub rows: &'a [LeaderRow<'a>],
    pub columns: &'a [LeaderColumn],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderRow<'a> {
    pub key: char,
    pub aliases: &'a [char],
    pub description: &'a str,
}

impl LeaderRow<'_> {
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
pub struct LeaderColumn {
    pub start: usize,
    pub len: usize,
    pub offset: usize,
    pub width: usize,
    pub key_width: usize,
    pub description_offset: usize,
}
