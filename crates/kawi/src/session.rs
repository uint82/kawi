use anyhow::{Context, Result};
use kawi_editor::ViewState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::persist::write_atomically;
use crate::text::one_line;
use crate::workspace;

mod folds;

pub use folds::{SessionFold, SessionFoldMethod, SessionFolds};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folds: Option<SessionFolds>,
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
    Ok(sessions.join(format!("{}.json", workspace::key(&cwd))))
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
    validate_buffers(&session.buffers)?;
    let mut ids = BTreeSet::new();
    validate_window(&session.windows, session.buffers.len(), 1, &mut ids)?;
    if !ids.contains(&session.active_window) {
        return Err("active window does not exist".to_string());
    }
    Ok(())
}

fn validate_buffers(buffers: &[SessionBuffer]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for buffer in buffers {
        if let Some(folds) = &buffer.folds {
            folds.validate()?;
        }
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
mod tests;
