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
                folds: None,
            },
            SessionBuffer {
                path: None,
                id: Some("second".to_string()),
                folds: None,
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
fn session_folds_round_trip_through_json() {
    let dir = temp_dir("fold_roundtrip");
    let path = dir.join("session.json");
    let mut session = sample();
    session.buffers[0].folds = Some(SessionFolds {
        method: SessionFoldMethod::Indent,
        content_hash: 42,
        manual: vec![SessionFold { start: 1, end: 3 }],
        closed: vec![1],
    });
    save(&path, &session).unwrap();
    assert_eq!(load(&path), Ok(Some(session)));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sessions_without_folds_load() {
    let dir = temp_dir("without_folds");
    let path = dir.join("session.json");
    let mut value = serde_json::to_value(sample()).unwrap();
    for buffer in value["buffers"].as_array_mut().unwrap() {
        buffer.as_object_mut().unwrap().remove("folds");
    }
    fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
    let loaded = load(&path).unwrap().expect("session");
    assert!(loaded.buffers.iter().all(|buffer| buffer.folds.is_none()));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_fold_ranges_are_rejected() {
    let mut session = sample();
    session.buffers[0].folds = Some(SessionFolds {
        method: SessionFoldMethod::Manual,
        content_hash: 7,
        manual: vec![SessionFold { start: 3, end: 2 }],
        closed: Vec::new(),
    });
    assert_eq!(
        validate(&session),
        Err("fold range is empty or reversed".to_string())
    );
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
