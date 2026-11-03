use super::*;
use crate::keymap::Flavor;
use kawi_buffer::Buffer;
use kawi_dap::Breakpoint;
use std::fs;
use std::sync::mpsc;

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_debug_app_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.rs");
    fs::write(&path, "one\ntwo\nthree\n").unwrap();
    path
}

fn app(path: &Path) -> App {
    App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    )
}

fn state_paths(root: &Path) -> StatePaths {
    StatePaths {
        config: root.join("config.toml"),
        config_log: root.join("config.log"),
        keymaps: root.join("keymaps.toml"),
        plugins: root.join("plugins"),
        installed_plugins: root.join("installed-plugins"),
        plugin_lock: root.join("plugins.lock"),
        plugin_settings: root.join("plugins.toml"),
        languages: root.join("languages.toml"),
        debug: root.join("debug.toml"),
        tasks: root.join("tasks.toml"),
        themes: root.join("themes"),
        queries: root.join("queries"),
        history: root.join("history"),
        sessions: root.join("sessions"),
        views: root.join("views"),
        recovery: root.join("recovery"),
        undo: root.join("undo"),
    }
}

#[test]
fn load_reports_debug_config_warnings() {
    let root = temp_file("warnings").parent().unwrap().to_path_buf();
    let paths = state_paths(&root);
    fs::write(&paths.debug, "[adapter.fake]\ncommand = \"\"\n").unwrap();
    let (wake, _wake_rx) = mpsc::channel();
    let mut debug = DebugUi::default();
    let warnings = debug.load(&paths, root.clone(), wake);
    assert!(!warnings.is_empty());
    assert!(warnings[0].contains("non-empty string"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn toggling_a_breakpoint_updates_signs() {
    let path = temp_file("breakpoint");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    assert!(app.debug.toggle_breakpoint(&app.editor));
    assert_eq!(app.debug.sign_for(&path, 0), Some(DebugSign::Unverified));
    app.debug.apply_event(DapEvent::Breakpoint {
        reason: "changed".to_string(),
        breakpoint: Breakpoint {
            verified: true,
            line: Some(1),
            source: Some(path.clone()),
            message: None,
        },
    });
    assert_eq!(app.debug.sign_for(&path, 0), Some(DebugSign::Breakpoint));
    assert!(app.debug.toggle_breakpoint(&app.editor));
    assert_eq!(app.debug.sign_for(&path, 0), None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stopped_event_jumps_to_the_top_frame() {
    let path = temp_file("jump");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    app.debug.apply_event(DapEvent::Stopped {
        reason: "breakpoint".to_string(),
        thread_id: Some(1),
        all_threads_stopped: true,
        description: None,
        text: Some("paused".to_string()),
    });
    app.debug.apply_event(DapEvent::StackTrace {
        thread_id: 1,
        frames: vec![StackFrame {
            id: 1,
            name: "main".to_string(),
            source: Some(path.clone()),
            line: 2,
            column: 3,
        }],
    });
    assert_eq!(
        app.debug.take_jump(),
        Some(DebugJump {
            path,
            line: 1,
            column: 2,
        })
    );
    assert_eq!(app.debug.display(), Some("dap: stopped"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn status_report_lists_breakpoints() {
    let path = temp_file("status");
    let root = path.parent().unwrap().to_path_buf();
    let mut app = app(&path);
    assert!(app.debug.toggle_breakpoint(&app.editor));
    app.debug.show_status();
    let report = app.debug.take_report().expect("report");
    assert_eq!(report.topic, "debug-status");
    assert!(report.lines.iter().any(|line| line == "breakpoints: 1"));
    let _ = fs::remove_dir_all(root);
}
