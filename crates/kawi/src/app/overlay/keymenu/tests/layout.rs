use kawi_tui::frame::{KeyRow, Rect};

use super::super::{keymenu_columns, keymenu_rect, KEYGUIDE_TITLE, LEADER_TITLE};

#[test]
fn short_menus_use_one_column() {
    let rows: Vec<KeyRow> = ('a'..='j')
        .map(|key| KeyRow {
            key,
            aliases: &[],
            description: "item",
        })
        .collect();
    let columns = keymenu_columns(&rows, 58);
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0].len, 10);
}

#[test]
fn growing_menus_use_two_columns() {
    let rows: Vec<KeyRow> = ('a'..='k')
        .map(|key| KeyRow {
            key,
            aliases: &[],
            description: "item",
        })
        .collect();
    let columns = keymenu_columns(&rows, 58);
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].start, 0);
    assert_eq!(columns[0].len, 6);
    assert_eq!(columns[1].start, 6);
    assert_eq!(columns[1].len, 5);
}

#[test]
fn wide_two_column_menus_fall_back_to_one_column() {
    let rows: Vec<KeyRow> = ('a'..='k')
        .map(|key| KeyRow {
            key,
            aliases: &[],
            description: "abcdefghijklmnopqrstuvwxyz1234",
        })
        .collect();
    let columns = keymenu_columns(&rows, 58);
    assert_eq!(columns.len(), 1);
}

#[test]
fn menu_is_right_aligned_above_the_bottom_edge() {
    let rows = [
        KeyRow {
            key: 'f',
            aliases: &[],
            description: "find",
        },
        KeyRow {
            key: 'v',
            aliases: &[],
            description: "git",
        },
    ];
    let columns = keymenu_columns(&rows, 58);
    let rect = keymenu_rect(Rect::new(0, 0, 80, 24), LEADER_TITLE, "", None, &columns);
    assert_eq!(rect.width, 9);
    assert_eq!(rect.height, 5);
    assert_eq!(rect.x, 70);
    assert_eq!(rect.y, 18);
}

#[test]
fn aliases_widen_the_key_column() {
    let rows = [KeyRow {
        key: 'f',
        aliases: &['p'],
        description: "project files",
    }];
    let columns = keymenu_columns(&rows, 58);
    assert_eq!(columns[0].key_width, 3);
    assert_eq!(columns[0].description_offset, 5);
}

#[test]
fn hint_menu_reserves_one_row() {
    let rect = keymenu_rect(
        Rect::new(0, 0, 80, 24),
        KEYGUIDE_TITLE,
        "f",
        Some("target character"),
        &[],
    );
    assert_eq!(rect.height, 4);
    assert_eq!(rect.width, 18);
}
