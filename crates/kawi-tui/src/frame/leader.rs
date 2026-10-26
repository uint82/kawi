use super::Rect;

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
    pub description: &'a str,
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
