use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use super::entry::{Entry, EntryKind};
use super::name::{escape_name, unescape_name};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app) enum Change {
    Create {
        path: PathBuf,
        kind: EntryKind,
    },
    Rename {
        id: u64,
        from: PathBuf,
        to: PathBuf,
        kind: EntryKind,
    },
    Copy {
        id: u64,
        from: PathBuf,
        to: PathBuf,
        kind: EntryKind,
    },
    Delete {
        id: u64,
        path: PathBuf,
        kind: EntryKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DirectoryLine {
    pub(super) id: Option<u64>,
    pub(super) text: String,
}

pub(super) fn diff(
    root: &Path,
    entries: &[Entry],
    lines: &[DirectoryLine],
) -> Result<Vec<Change>, String> {
    let mut by_id = HashMap::with_capacity(entries.len());
    by_id.extend(entries.iter().map(|entry| (entry.id, entry)));
    let mut targets = HashSet::with_capacity(lines.len());
    let mut seen = HashSet::with_capacity(entries.len());
    let mut changes = Vec::with_capacity(entries.len());
    for (index, line) in lines.iter().enumerate() {
        if line.text.is_empty() {
            continue;
        }
        match line.id {
            Some(id) => {
                let entry = by_id
                    .get(&id)
                    .ok_or_else(|| format!("line {} has an unknown entry id", index + 1))?;
                let from = root.join(&entry.name);
                let target = if entry.display_name_matches(&line.text) {
                    None
                } else {
                    Some(target_for_entry(root, entry, &line.text)?)
                };
                record_target(&mut targets, target.as_deref().unwrap_or(&from))?;
                if !seen.insert(id) {
                    let to = target.unwrap_or_else(|| from.clone());
                    changes.push(Change::Copy {
                        id,
                        from,
                        to,
                        kind: entry.kind,
                    });
                    continue;
                }
                if let Some(target) = target {
                    if from != target {
                        changes.push(Change::Rename {
                            id,
                            from,
                            to: target,
                            kind: entry.kind,
                        });
                    }
                }
            }
            None => {
                let (target, kind) = target_for_new_line(root, &line.text)?;
                record_target(&mut targets, &target)?;
                changes.push(Change::Create { path: target, kind });
            }
        }
    }
    for entry in entries {
        if !seen.contains(&entry.id) {
            changes.push(Change::Delete {
                id: entry.id,
                path: root.join(&entry.name),
                kind: entry.kind,
            });
        }
    }
    Ok(changes)
}

fn target_for_entry(root: &Path, entry: &Entry, text: &str) -> Result<PathBuf, String> {
    let (relative, is_directory) = parse_text(text)?;
    let expects_directory = entry.kind == EntryKind::Directory;
    if is_directory != expects_directory {
        let expected = if expects_directory {
            "a trailing slash"
        } else {
            "no trailing slash"
        };
        return Err(format!(
            "{} must use {expected}",
            escape_name(entry.name.as_os_str())
        ));
    }
    Ok(root.join(relative))
}

fn target_for_new_line(root: &Path, text: &str) -> Result<(PathBuf, EntryKind), String> {
    let (relative, is_directory) = parse_text(text)?;
    let kind = if is_directory {
        EntryKind::Directory
    } else {
        EntryKind::File
    };
    Ok((root.join(relative), kind))
}

pub(super) fn parse_text(text: &str) -> Result<(PathBuf, bool), String> {
    let (name, is_directory) = split_name(text);
    if name.is_empty() {
        return Err("entry name is empty".to_string());
    }
    let path = PathBuf::from(unescape_name(name)?);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("invalid entry name {name}"));
    }
    Ok((path, is_directory))
}

fn split_name(text: &str) -> (&str, bool) {
    match text.strip_suffix('/') {
        Some(name) => (name, true),
        None => (text, false),
    }
}

fn record_target(targets: &mut HashSet<PathBuf>, target: &Path) -> Result<(), String> {
    if !targets.insert(target.to_path_buf()) {
        return Err(format!("duplicate target {}", target.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::*;
    use crate::app::directory::entry::EntryKind;

    fn root() -> PathBuf {
        PathBuf::from("/project")
    }

    fn entry(id: u64, name: &str, kind: EntryKind) -> Entry {
        Entry {
            id,
            name: OsString::from(name),
            kind,
            symlink_target: None,
            hidden: false,
            metadata: None,
        }
    }

    fn line(id: Option<u64>, text: &str) -> DirectoryLine {
        DirectoryLine {
            id,
            text: text.to_string(),
        }
    }

    #[test]
    fn unchanged_lines_produce_no_changes() {
        let entries = [entry(1, "a.txt", EntryKind::File)];
        assert!(diff(&root(), &entries, &[line(Some(1), "a.txt")])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn renamed_moved_and_created_lines_are_distinguished() {
        let entries = [
            entry(1, "a.txt", EntryKind::File),
            entry(2, "old", EntryKind::Directory),
        ];
        let changes = diff(
            &root(),
            &entries,
            &[
                line(Some(1), "sub/a.txt"),
                line(Some(2), "new/"),
                line(None, "sub/new.txt"),
            ],
        )
        .unwrap();
        assert!(changes.contains(&Change::Rename {
            id: 1,
            from: root().join("a.txt"),
            to: root().join("sub/a.txt"),
            kind: EntryKind::File,
        }));
        assert!(changes.contains(&Change::Rename {
            id: 2,
            from: root().join("old"),
            to: root().join("new"),
            kind: EntryKind::Directory,
        }));
        assert!(changes.contains(&Change::Create {
            path: root().join("sub/new.txt"),
            kind: EntryKind::File,
        }));
    }

    #[test]
    fn missing_ids_are_deleted_and_repeated_ids_are_copied() {
        let entries = [
            entry(1, "a.txt", EntryKind::File),
            entry(2, "b.txt", EntryKind::File),
        ];
        let changes = diff(
            &root(),
            &entries,
            &[line(Some(1), "a.txt"), line(Some(1), "copy.txt")],
        )
        .unwrap();
        assert!(changes.contains(&Change::Copy {
            id: 1,
            from: root().join("a.txt"),
            to: root().join("copy.txt"),
            kind: EntryKind::File,
        }));
        assert!(changes.contains(&Change::Delete {
            id: 2,
            path: root().join("b.txt"),
            kind: EntryKind::File,
        }));
    }

    #[test]
    fn escaped_names_are_decoded() {
        assert_eq!(parse_text(r"a\nb").unwrap(), (PathBuf::from("a\nb"), false));
        assert_eq!(parse_text(r"a\\b").unwrap(), (PathBuf::from("a\\b"), false));
    }

    #[test]
    fn escaped_unchanged_lines_produce_no_changes() {
        let entries = [
            entry(1, "a\nb", EntryKind::File),
            entry(2, "a\\b", EntryKind::File),
        ];
        assert!(diff(
            &root(),
            &entries,
            &[line(Some(1), r"a\nb"), line(Some(2), r"a\\b")]
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn invalid_names_and_kind_changes_are_rejected() {
        let entries = [entry(1, "dir", EntryKind::Directory)];
        assert!(diff(&root(), &entries, &[line(Some(1), "dir")]).is_err());
        assert!(diff(&root(), &entries, &[line(None, "../x")]).is_err());
        assert!(diff(&root(), &entries, &[line(None, "/x")]).is_err());
        assert!(diff(&root(), &entries, &[line(None, "one/")]).is_ok());
    }

    #[test]
    fn duplicate_targets_are_rejected() {
        let entries = [entry(1, "a.txt", EntryKind::File)];
        let error = diff(
            &root(),
            &entries,
            &[line(Some(1), "same.txt"), line(None, "same.txt")],
        )
        .unwrap_err();
        assert!(error.contains("duplicate target"));
    }
}
