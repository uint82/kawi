use anyhow::{Context, Result};
use kawi_editor::ViewState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::persist::write_atomically;
use crate::text::one_line;

pub(crate) const SESSION_VERSION: u32 = 2;
pub(crate) const MIN_SESSION_VERSION: u32 = 1;
pub(crate) const MAX_WINDOW_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub cwd: String,
    pub buffers: Vec<SessionBuffer>,
    pub windows: SessionWindow,
    pub active_window: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBuffer {
    pub path: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionView {
    pub row: usize,
    pub col: usize,
    pub desired: usize,
    pub top: usize,
    pub leftcol: usize,
}

impl From<ViewState> for SessionView {
    fn from(view: ViewState) -> Self {
        Self {
            row: view.row,
            col: view.col,
            desired: view.desired,
            top: view.top,
            leftcol: view.leftcol,
        }
    }
}

impl From<SessionView> for ViewState {
    fn from(view: SessionView) -> Self {
        Self {
            row: view.row,
            col: view.col,
            desired: view.desired,
            top: view.top,
            leftcol: view.leftcol,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionWindow {
    Leaf {
        id: u64,
        buffer: usize,
        view: SessionView,
    },
    Split {
        direction: SessionDirection,
        ratio: u16,
        first: Box<SessionWindow>,
        second: Box<SessionWindow>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionDirection {
    Horizontal,
    Vertical,
}

pub fn load_for_cwd(sessions: &Path, cwd: &Path) -> Result<Option<Session>, String> {
    load(&session_path(sessions, cwd)?)
}

pub fn save_for_cwd(sessions: &Path, cwd: &Path, session: &Session) -> Result<()> {
    let path = session_path(sessions, cwd).map_err(anyhow::Error::msg)?;
    save(&path, session)
}

fn session_path(sessions: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let cwd = fs::canonicalize(cwd).map_err(|error| format!("session: {error}"))?;
    Ok(sessions.join(format!("{}.json", workspace_key(&cwd))))
}

fn workspace_key(cwd: &Path) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(cwd.as_os_str().as_encoded_bytes());
    let mut key = String::with_capacity(64);
    for byte in digest {
        key.push(HEX[(byte >> 4) as usize] as char);
        key.push(HEX[(byte & 15) as usize] as char);
    }
    key
}

fn load(path: &Path) -> Result<Option<Session>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("session.json: {error}")),
    };
    let session: Session = serde_json::from_str(&text)
        .map_err(|error| format!("session.json: {}", one_line(&error.to_string())))?;
    validate(&session).map_err(|error| format!("session.json: {error}"))?;
    Ok(Some(session))
}

fn save(path: &Path, session: &Session) -> Result<()> {
    let text = serde_json::to_string_pretty(session).context("serialize session")?;
    write_atomically(path, &format!("{text}\n"))
}

fn validate(session: &Session) -> Result<(), String> {
    if !(MIN_SESSION_VERSION..=SESSION_VERSION).contains(&session.version) {
        return Err(format!("unsupported version {}", session.version));
    }
    if session.buffers.is_empty() {
        return Err("session has no buffers".to_string());
    }
    validate_buffer_ids(&session.buffers)?;
    let mut ids = BTreeSet::new();
    validate_window(&session.windows, session.buffers.len(), 1, &mut ids)?;
    if !ids.contains(&session.active_window) {
        return Err("active window does not exist".to_string());
    }
    Ok(())
}

fn validate_buffer_ids(buffers: &[SessionBuffer]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for buffer in buffers {
        let Some(id) = buffer.id.as_deref() else {
            continue;
        };
        if id.is_empty() {
            return Err("buffer id is empty".to_string());
        }
        if !ids.insert(id) {
            return Err(format!("duplicate buffer id {id}"));
        }
    }
    Ok(())
}

fn validate_window(
    window: &SessionWindow,
    buffer_count: usize,
    depth: usize,
    ids: &mut BTreeSet<u64>,
) -> Result<usize, String> {
    if depth > MAX_WINDOW_DEPTH {
        return Err("window nesting is too deep".to_string());
    }
    match window {
        SessionWindow::Leaf { id, buffer, .. } => {
            if !ids.insert(*id) {
                return Err(format!("duplicate window id {id}"));
            }
            if *buffer >= buffer_count {
                return Err(format!("window {id} references missing buffer {buffer}"));
            }
            Ok(1)
        }
        SessionWindow::Split { first, second, .. } => {
            let first = validate_window(first, buffer_count, depth + 1, ids)?;
            let second = validate_window(second, buffer_count, depth + 1, ids)?;
            Ok(first.saturating_add(second))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("kawi_session_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn view(row: usize, top: usize) -> SessionView {
        SessionView {
            row,
            col: 1,
            desired: 2,
            top,
            leftcol: 3,
        }
    }

    fn sample() -> Session {
        Session {
            version: SESSION_VERSION,
            cwd: "/tmp/kawi".to_string(),
            buffers: vec![
                SessionBuffer {
                    path: Some("/tmp/kawi/a.txt".to_string()),
                    id: Some("first".to_string()),
                },
                SessionBuffer {
                    path: None,
                    id: Some("second".to_string()),
                },
            ],
            windows: SessionWindow::Split {
                direction: SessionDirection::Vertical,
                ratio: 60,
                first: Box::new(SessionWindow::Leaf {
                    id: 1,
                    buffer: 0,
                    view: view(4, 2),
                }),
                second: Box::new(SessionWindow::Leaf {
                    id: 2,
                    buffer: 1,
                    view: view(0, 0),
                }),
            },
            active_window: 2,
        }
    }

    #[test]
    fn session_round_trips_through_json() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("session.json");
        let session = sample();
        save(&path, &session).unwrap();
        assert_eq!(load(&path), Ok(Some(session)));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_session_file_is_absent() {
        let dir = temp_dir("missing");
        assert_eq!(load(&dir.join("session.json")), Ok(None));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn version_one_sessions_load_without_buffer_ids() {
        let dir = temp_dir("version_one");
        let path = dir.join("session.json");
        let mut value = serde_json::to_value(sample()).unwrap();
        value["version"] = serde_json::json!(1);
        for buffer in value["buffers"].as_array_mut().unwrap() {
            buffer.as_object_mut().unwrap().remove("id");
        }
        fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
        let loaded = load(&path).unwrap().expect("session");
        assert_eq!(loaded.version, 1);
        assert!(loaded.buffers.iter().all(|buffer| buffer.id.is_none()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_session_is_rejected() {
        let dir = temp_dir("corrupt");
        let path = dir.join("session.json");
        fs::write(&path, "{").unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.starts_with("session.json: "));
        assert!(!error.contains('\n'));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unsupported_versions_are_rejected() {
        let mut session = sample();
        session.version = SESSION_VERSION + 1;
        assert_eq!(validate(&session), Err("unsupported version 3".to_string()));
    }

    #[test]
    fn legacy_sessions_are_accepted() {
        let mut session = sample();
        session.version = MIN_SESSION_VERSION;
        for buffer in &mut session.buffers {
            buffer.id = None;
        }
        assert_eq!(validate(&session), Ok(()));
    }

    #[test]
    fn duplicate_buffer_ids_are_rejected() {
        let mut session = sample();
        session.buffers[1].id = session.buffers[0].id.clone();
        assert_eq!(
            validate(&session),
            Err("duplicate buffer id first".to_string())
        );
    }

    #[test]
    fn duplicate_window_ids_are_rejected() {
        let mut session = sample();
        session.windows = SessionWindow::Split {
            direction: SessionDirection::Horizontal,
            ratio: 50,
            first: Box::new(SessionWindow::Leaf {
                id: 3,
                buffer: 0,
                view: view(0, 0),
            }),
            second: Box::new(SessionWindow::Leaf {
                id: 3,
                buffer: 1,
                view: view(0, 0),
            }),
        };
        assert_eq!(validate(&session), Err("duplicate window id 3".to_string()));
    }

    #[test]
    fn missing_buffer_references_are_rejected() {
        let mut session = sample();
        session.windows = SessionWindow::Leaf {
            id: 1,
            buffer: 2,
            view: view(0, 0),
        };
        assert_eq!(
            validate(&session),
            Err("window 1 references missing buffer 2".to_string())
        );
    }

    #[test]
    fn active_window_must_exist() {
        let mut session = sample();
        session.active_window = 9;
        assert_eq!(
            validate(&session),
            Err("active window does not exist".to_string())
        );
    }

    #[test]
    fn workspaces_get_distinct_session_files() {
        let root = temp_dir("workspaces");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let sessions = root.join("sessions");
        let first_path = session_path(&sessions, &first).unwrap();
        let second_path = session_path(&sessions, &second).unwrap();
        assert_ne!(first_path, second_path);
        assert_eq!(
            first_path,
            session_path(&sessions, &first.join(".")).unwrap()
        );
        assert_eq!(
            first_path.extension().and_then(|value| value.to_str()),
            Some("json")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_sessions_round_trip_independently() {
        let root = temp_dir("independent");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let sessions = root.join("sessions");
        let mut first_session = sample();
        first_session.cwd = first.to_string_lossy().into_owned();
        let mut second_session = sample();
        second_session.cwd = second.to_string_lossy().into_owned();
        second_session.active_window = 1;
        save_for_cwd(&sessions, &first, &first_session).unwrap();
        save_for_cwd(&sessions, &second, &second_session).unwrap();
        assert_eq!(load_for_cwd(&sessions, &first), Ok(Some(first_session)));
        assert_eq!(load_for_cwd(&sessions, &second), Ok(Some(second_session)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_view_converts_both_ways() {
        let state = ViewState {
            row: 4,
            col: 3,
            desired: 2,
            top: 1,
            leftcol: 0,
        };
        let view = SessionView::from(state);
        assert_eq!(ViewState::from(view), state);
    }
}
