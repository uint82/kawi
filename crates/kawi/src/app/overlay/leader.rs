use kawi_tui::frame::{LeaderColumn, LeaderMenu, LeaderRow, Rect};
use kawi_tui::overlay::PANEL_BORDER;
use kawi_tui::text::{char_width, display_width};

use super::super::App;
use super::{OverlayData, MAX_LIST_WIDTH};
use crate::keymap::Keymap;
use crate::leader::{self, LeaderPath};

const SINGLE_COLUMN_LIMIT: usize = 10;
const KEY_DESCRIPTION_GAP: usize = 2;
const COLUMN_GAP: usize = 2;
const RIGHT_MARGIN: usize = 1;
const BOTTOM_MARGIN: usize = 1;
const TITLE: &str = "Leader";

pub(super) struct LeaderData {
    rect: Rect,
    group: Option<char>,
    rows: Vec<LeaderRow<'static>>,
    columns: Vec<LeaderColumn>,
}

impl LeaderData {
    pub(super) fn view(&self) -> LeaderMenu<'_> {
        LeaderMenu {
            rect: self.rect,
            title: TITLE,
            group: self.group,
            rows: &self.rows,
            columns: &self.columns,
        }
    }
}

impl App {
    pub(super) fn leader_overlay(&self) -> Option<OverlayData<'_>> {
        let path = self.keymap.leader_path()?;
        let visual = self.editor.mode().is_visual();
        let rows = leader_rows(path, visual);
        let group = leader_group(path);
        let area = self.window_area();
        let columns = leader_columns(&rows, leader_width_limit(area));
        let rect = leader_rect(area, group, &columns);
        Some(OverlayData {
            list: None,
            panel: None,
            leader: Some(LeaderData {
                rect,
                group,
                rows,
                columns,
            }),
        })
    }
}

fn leader_rows(path: LeaderPath, visual: bool) -> Vec<LeaderRow<'static>> {
    match path {
        LeaderPath::Root => leader::groups(visual)
            .map(|group| leader_row(group.key, group.label))
            .collect(),
        LeaderPath::Group(group) => group
            .commands
            .iter()
            .map(|command| leader_row(command.key, command.label))
            .collect(),
    }
}

fn leader_group(path: LeaderPath) -> Option<char> {
    match path {
        LeaderPath::Root => None,
        LeaderPath::Group(group) => Some(group.key),
    }
}

fn leader_row(key: char, description: &'static str) -> LeaderRow<'static> {
    LeaderRow { key, description }
}

fn leader_columns(rows: &[LeaderRow<'_>], max_width: usize) -> Vec<LeaderColumn> {
    let key_width = leader_key_width(rows);
    let single = build_columns(rows, 1, key_width);
    if rows.len() <= SINGLE_COLUMN_LIMIT {
        return single;
    }
    let double = build_columns(rows, 2, key_width);
    if leader_content_width(&double) <= max_width {
        double
    } else {
        single
    }
}

fn leader_key_width(rows: &[LeaderRow<'_>]) -> usize {
    rows.iter()
        .map(|row| char_width(row.key))
        .max()
        .unwrap_or(1)
        .max(1)
}

fn build_columns(rows: &[LeaderRow<'_>], count: usize, key_width: usize) -> Vec<LeaderColumn> {
    let per_column = rows.len().div_ceil(count).max(1);
    let description_offset = key_width.saturating_add(KEY_DESCRIPTION_GAP);
    let mut offset = 0;
    rows.chunks(per_column)
        .enumerate()
        .map(|(index, chunk)| {
            let description_width = chunk
                .iter()
                .map(|row| display_width(row.description))
                .max()
                .unwrap_or(0);
            let width = description_offset.saturating_add(description_width);
            let column = LeaderColumn {
                start: index.saturating_mul(per_column),
                len: chunk.len(),
                offset,
                width,
                key_width,
                description_offset,
            };
            offset = offset.saturating_add(width).saturating_add(COLUMN_GAP);
            column
        })
        .collect()
}

fn leader_content_width(columns: &[LeaderColumn]) -> usize {
    columns
        .last()
        .map(|column| column.offset.saturating_add(column.width))
        .unwrap_or(0)
}

fn leader_width_limit(area: Rect) -> usize {
    MAX_LIST_WIDTH
        .saturating_sub(PANEL_BORDER)
        .min(area.width.saturating_sub(PANEL_BORDER + RIGHT_MARGIN))
        .max(1)
}

fn leader_rect(area: Rect, group: Option<char>, columns: &[LeaderColumn]) -> Rect {
    let title_width = display_width(TITLE).saturating_add(match group {
        Some(key) => char_width(key).saturating_add(1),
        None => 0,
    });
    let content_width = leader_content_width(columns).max(title_width);
    let rows_height = columns.iter().map(|column| column.len).max().unwrap_or(0);
    let width = content_width
        .saturating_add(PANEL_BORDER)
        .min(MAX_LIST_WIDTH)
        .min(area.width)
        .max(1);
    let height = rows_height
        .saturating_add(1)
        .saturating_add(PANEL_BORDER)
        .min(area.height)
        .max(1);
    let x = area
        .right()
        .saturating_sub(width)
        .saturating_sub(RIGHT_MARGIN)
        .max(area.x);
    let y = area
        .bottom()
        .saturating_sub(height)
        .saturating_sub(BOTTOM_MARGIN)
        .max(area.y);
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests;
