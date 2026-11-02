use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::PathBuf;

use super::diff::DirectoryLine;
use super::session::*;
use crate::app::directory::entry::{Entry, EntryKind};
use crate::app::directory::sort::SortOptions;

fn state() -> DirectoryBuffer {
    DirectoryBuffer {
        root: PathBuf::from("/project"),
        show_hidden: false,
        sort: SortOptions::default(),
        original: vec![
            Entry {
                id: 1,
                name: OsString::from("a"),
                kind: EntryKind::File,
                symlink_target: None,
                hidden: false,
                metadata: None,
            },
            Entry {
                id: 2,
                name: OsString::from("b"),
                kind: EntryKind::File,
                symlink_target: None,
                hidden: false,
                metadata: None,
            },
        ],
        lines: vec![
            DirectoryLine {
                id: Some(1),
                text: "a".to_string(),
            },
            DirectoryLine {
                id: Some(2),
                text: "b".to_string(),
            },
        ],
        next_id: 3,
        revision: 0,
        removed: BTreeMap::new(),
        positions: HashMap::new(),
        metadata_widths: Default::default(),
        external_changed: false,
        deleted: false,
        view_options_pending: false,
    }
}

#[test]
fn renamed_lines_keep_identity() {
    let mut state = state();
    state.sync(vec!["alpha".to_string(), "b".to_string()], 1);
    assert_eq!(state.lines[0].id, Some(1));
    assert_eq!(state.lines[1].id, Some(2));
}

#[test]
fn moved_lines_keep_identity_across_a_cut() {
    let mut state = state();
    state.sync(vec!["b".to_string()], 1);
    state.sync(vec!["b".to_string(), "a".to_string()], 2);
    assert_eq!(state.lines[0].id, Some(2));
    assert_eq!(state.lines[1].id, Some(1));
}

#[test]
fn duplicates_share_identity() {
    let mut state = state();
    state.sync(vec!["a".to_string(), "a".to_string()], 1);
    assert_eq!(state.lines[0].id, Some(1));
    assert_eq!(state.lines[1].id, Some(1));
}

#[test]
fn inserted_lines_are_new() {
    let mut state = state();
    state.sync(vec!["a".to_string(), "b".to_string(), "c".to_string()], 1);
    assert_eq!(state.lines[2].id, None);
}

#[test]
fn deleted_lines_are_forgotten() {
    let mut state = state();
    state.sync(vec!["a".to_string()], 1);
    assert!(state.removed.contains_key("b"));
}

#[test]
fn renamed_copies_keep_the_source_identity() {
    let mut state = state();
    state.sync(vec!["a".to_string(), "b".to_string(), "a".to_string()], 1);
    state.sync(vec!["a".to_string(), "b".to_string(), "c".to_string()], 2);
    assert_eq!(state.lines[2].id, Some(1));
}

#[test]
fn blank_lines_keep_row_alignment() {
    let mut state = state();
    state.sync(vec!["a".to_string(), String::new(), "b".to_string()], 1);
    assert_eq!(state.lines.len(), 3);
    assert_eq!(state.lines[0].id, Some(1));
    assert_eq!(state.lines[1].id, None);
    assert_eq!(state.lines[2].id, Some(2));
}
