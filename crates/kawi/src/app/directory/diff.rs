use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use super::entry::{Entry, EntryKind};

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
    let by_id: HashMap<u64, &Entry> = entries.iter().map(|entry| (entry.id, entry)).collect();
    let mut targets = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut changes = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.text.is_empty() {
            continue;
        }
        match line.id {
            Some(id) => {
                let entry = by_id
                    .get(&id)
                    .ok_or_else(|| format!("line {} has an unknown entry id", index + 1))?;
                let (target, kind) = target_for_line(root, entry, &line.text)?;
                record_target(&mut targets, &target, index)?;
                if !seen.insert(id) {
                    changes.push(Change::Copy {
                        id,
                        from: root.join(&entry.name),
                        to: target,
                        kind,
                    });
                    continue;
                }
                let from = root.join(&entry.name);
                if from != target {
                    changes.push(Change::Rename {
                        id,
                        from,
                        to: target,
                        kind,
                    });
                }
            }
            None => {
                let (target, kind) = target_for_new_line(root, &line.text)?;
                record_target(&mut targets, &target, index)?;
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

fn target_for_line(root: &Path, entry: &Entry, text: &str) -> Result<(PathBuf, EntryKind), String> {
    if text == entry.line_text() {
        return Ok((root.join(&entry.name), entry.kind));
    }
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
            entry.name.to_string_lossy()
        ));
    }
    Ok((root.join(relative), entry.kind))
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
    let is_directory = text.ends_with('/');
    let name = if is_directory {
        &text[..text.len().saturating_sub(1)]
    } else {
        text
    };
    if name.is_empty() {
        return Err("entry name is empty".to_string());
    }
    if name.contains('\0') {
        return Err("entry name contains a NUL byte".to_string());
    }
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("invalid entry name {name}"));
    }
    Ok((PathBuf::from(OsString::from(name)), is_directory))
}

fn record_target(
    targets: &mut BTreeMap<PathBuf, usize>,
    target: &Path,
    index: usize,
) -> Result<(), String> {
    if targets.insert(target.to_path_buf(), index).is_some() {
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
