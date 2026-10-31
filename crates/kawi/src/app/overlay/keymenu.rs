use kawi_tui::frame::{KeyColumn, KeyMenu, KeyRow, Rect};
use kawi_tui::overlay::PANEL_BORDER;
use kawi_tui::text::display_width;

use super::super::App;
use super::OverlayData;
use crate::leader::{self, LeaderPath};
use crate::vim::guide::GuideRow;

const KEYMENU_MAX_WIDTH: usize = 72;
const SINGLE_COLUMN_LIMIT: usize = 10;
const KEY_DESCRIPTION_GAP: usize = 2;
const COLUMN_GAP: usize = 2;
const RIGHT_MARGIN: usize = 1;
const BOTTOM_MARGIN: usize = 1;
const LEADER_TITLE: &str = "Leader";
const KEYGUIDE_TITLE: &str = "Keys";

pub(super) struct KeyMenuData {
    rect: Rect,
    title: &'static str,
    breadcrumb: String,
    hint: Option<&'static str>,
    rows: Vec<KeyRow<'static>>,
    columns: Vec<KeyColumn>,
}

impl KeyMenuData {
    pub(super) fn view(&self) -> KeyMenu<'_> {
        KeyMenu {
            rect: self.rect,
            title: self.title,
            breadcrumb: &self.breadcrumb,
            hint: self.hint,
            rows: &self.rows,
            columns: &self.columns,
        }
    }
}

impl App {
    pub(super) fn leader_overlay(&self) -> Option<OverlayData<'_>> {
        let path = self.pending_leader_path()?;
        let visual = self.editor.mode().is_visual();
        let rows = leader_rows(path, visual);
        let breadcrumb = leader_breadcrumb(path);
        let hint = None;
        let area = self.window_area();
        let columns = keymenu_columns(&rows, keymenu_width_limit(area));
        let rect = keymenu_rect(area, LEADER_TITLE, &breadcrumb, hint, &columns);
        Some(OverlayData {
            list: None,
            panel: None,
            key_menu: Some(KeyMenuData {
                rect,
                title: LEADER_TITLE,
                breadcrumb,
                hint,
                rows,
                columns,
            }),
        })
    }

    pub(super) fn keyguide_overlay(&self) -> Option<OverlayData<'_>> {
        let cache = self.keyguide_cache.as_ref()?;
        let area = self.window_area();
        let columns = keymenu_columns(&cache.rows, keymenu_width_limit(area));
        let rect = keymenu_rect(
            area,
            KEYGUIDE_TITLE,
            &cache.breadcrumb,
            cache.hint,
            &columns,
        );
        Some(OverlayData {
            list: None,
            panel: None,
            key_menu: Some(KeyMenuData {
                rect,
                title: KEYGUIDE_TITLE,
                breadcrumb: cache.breadcrumb.clone(),
                hint: cache.hint,
                rows: cache.rows.clone(),
                columns,
            }),
        })
    }
}

fn leader_rows(path: LeaderPath, visual: bool) -> Vec<KeyRow<'static>> {
    match path {
        LeaderPath::Root => leader::groups(visual)
            .map(|group| key_row(group.key, &[], group.label))
            .collect(),
        LeaderPath::Group(group) => group
            .commands
            .iter()
            .map(|command| key_row(command.key, command.aliases, command.label))
            .collect(),
    }
}

fn leader_breadcrumb(path: LeaderPath) -> String {
    match path {
        LeaderPath::Root => String::new(),
        LeaderPath::Group(group) => group.key.to_string(),
    }
}

pub(crate) fn guide_key_row(row: GuideRow) -> KeyRow<'static> {
    key_row(row.key, row.aliases, row.description)
}

fn key_row(key: char, aliases: &'static [char], description: &'static str) -> KeyRow<'static> {
    KeyRow {
        key,
        aliases,
        description,
    }
}

fn keymenu_columns(rows: &[KeyRow<'_>], max_width: usize) -> Vec<KeyColumn> {
    let key_width = keymenu_key_width(rows);
    let single = build_columns(rows, 1, key_width);
    if rows.len() <= SINGLE_COLUMN_LIMIT {
        return single;
    }
    let double = build_columns(rows, 2, key_width);
    if keymenu_content_width(&double) <= max_width {
        double
    } else {
        single
    }
}

fn keymenu_key_width(rows: &[KeyRow<'_>]) -> usize {
    rows.iter().map(KeyRow::key_width).max().unwrap_or(1).max(1)
}

fn build_columns(rows: &[KeyRow<'_>], count: usize, key_width: usize) -> Vec<KeyColumn> {
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
            let column = KeyColumn {
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

fn keymenu_content_width(columns: &[KeyColumn]) -> usize {
    columns
        .last()
        .map(|column| column.offset.saturating_add(column.width))
        .unwrap_or(0)
}

fn keymenu_width_limit(area: Rect) -> usize {
    KEYMENU_MAX_WIDTH
        .saturating_sub(PANEL_BORDER)
        .min(area.width.saturating_sub(PANEL_BORDER + RIGHT_MARGIN))
        .max(1)
}

fn keymenu_rect(
    area: Rect,
    title: &str,
    breadcrumb: &str,
    hint: Option<&str>,
    columns: &[KeyColumn],
) -> Rect {
    let title_width = keymenu_title_width(title, breadcrumb);
    let hint_width = hint.map(display_width).unwrap_or(0);
    let content_width = keymenu_content_width(columns)
        .max(title_width)
        .max(hint_width);
    let rows_height = columns.iter().map(|column| column.len).max().unwrap_or(0);
    let hint_height = usize::from(hint.is_some());
    let width = content_width
        .saturating_add(PANEL_BORDER)
        .min(KEYMENU_MAX_WIDTH)
        .min(area.width)
        .max(1);
    let height = rows_height
        .saturating_add(1)
        .saturating_add(hint_height)
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

fn keymenu_title_width(title: &str, breadcrumb: &str) -> usize {
    let title_width = display_width(title);
    if breadcrumb.is_empty() {
        title_width
    } else {
        title_width
            .saturating_add(1)
            .saturating_add(display_width(breadcrumb))
    }
}

#[cfg(test)]
mod tests;
