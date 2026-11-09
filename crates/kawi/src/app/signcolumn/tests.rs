use super::*;
use crate::app::git::GitSession;
use crate::keymap::Flavor;
use crate::options::SignColumnMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_git::GitFile;
use kawi_plugin::{Decoration, DecorationRef, DecorationScope, DecorationStyle};
use kawi_tui::frame::{DebugSign, GitSign, PluginSign};
use std::fs;
use std::path::PathBuf;

fn app_with_untracked_file() -> App {
    let file = GitFile {
        root: PathBuf::from("."),
        path: PathBuf::from("script.py"),
        relpath: "script.py".to_string(),
        mode: "100644".to_string(),
        untracked: true,
        crlf: false,
        no_final_newline: false,
        head_no_final_newline: false,
        base: Vec::new(),
        head: Vec::new(),
    };
    let lines = vec!["print('hello')".to_string()];
    let session = GitSession::new(file, &lines, 1, false);
    let mut app = App::new(Buffer::from_string("print('hello')"), Flavor::Vim);
    app.handle_resize(80, 24).expect("resize");
    app.git = Some(session);
    app
}

#[test]
fn signcolumn_defaults_to_hidden() {
    let app = app_with_untracked_file();
    assert_eq!(app.signcolumn, SignColumnMode::Never);
    assert!(!app.git_signs);
    assert!(app.view_signs().is_none());
}

#[test]
fn never_hides_git_signs_even_when_enabled() {
    let mut app = app_with_untracked_file();
    app.git_signs = true;
    app.update_signcolumn_width();
    assert_eq!(app.editor.signcolumn_width(), 0);
    assert!(app.view_signs().is_none());
}

#[test]
fn auto_shows_git_signs_only_when_enabled_and_present() {
    let mut app = app_with_untracked_file();
    app.signcolumn = SignColumnMode::Auto;
    app.update_signcolumn_width();
    assert_eq!(app.editor.signcolumn_width(), 0);
    assert!(app.view_signs().is_none());

    app.git_signs = true;
    app.update_signcolumn_width();
    let signs = app.view_signs().expect("signs");
    assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
    assert_eq!(signs[0], Some(Sign::Git(GitSign::Untracked)));
}

#[test]
fn always_reserves_the_column_without_signs() {
    let mut app = app_with_untracked_file();
    app.signcolumn = SignColumnMode::Always;
    app.update_signcolumn_width();
    let signs = app.view_signs().expect("signs");
    assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
    assert!(signs.iter().all(Option::is_none));
}

#[test]
fn auto_shows_diagnostics_from_problem_state() {
    let dir = crate::test_support::temp_dir("diagnostic_sign");
    let path = dir.join("main.rs");
    fs::write(&path, "one\ntwo\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.handle_resize(80, 24).expect("resize");
    app.signcolumn = SignColumnMode::Auto;
    app.problems.replace_lsp_test(
        "fake",
        &path,
        vec![crate::test_support::diagnostic(
            0,
            kawi_lsp::DiagnosticSeverity::Error,
            "message",
        )],
    );
    app.update_signcolumn_width();
    assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
    let signs = app.view_signs().expect("signs");
    assert_eq!(
        signs[0],
        Some(Sign::Diagnostic(UiDiagnosticSeverity::Error))
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn insert_mode_hides_diagnostic_signs_and_keeps_the_column() {
    let dir = crate::test_support::temp_dir("diagnostic_sign_insert");
    let path = dir.join("main.rs");
    fs::write(&path, "one\ntwo\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.handle_resize(80, 24).expect("resize");
    app.signcolumn = SignColumnMode::Auto;
    app.problems.replace_lsp_test(
        "fake",
        &path,
        vec![crate::test_support::diagnostic(
            0,
            kawi_lsp::DiagnosticSeverity::Error,
            "message",
        )],
    );
    app.update_signcolumn_width();
    assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
    assert_eq!(
        app.view_signs().expect("signs")[0],
        Some(Sign::Diagnostic(UiDiagnosticSeverity::Error))
    );
    press_key(&mut app, KeyCode::Char('i'));
    assert_eq!(app.editor.signcolumn_width(), SIGN_COLUMN_WIDTH);
    assert_eq!(app.view_signs().expect("signs")[0], None);
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(
        app.view_signs().expect("signs")[0],
        Some(Sign::Diagnostic(UiDiagnosticSeverity::Error))
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn insert_mode_keeps_non_diagnostic_signs() {
    let mut app = app_with_untracked_file();
    app.signcolumn = SignColumnMode::Auto;
    app.git_signs = true;
    app.update_signcolumn_width();
    assert_eq!(
        app.view_signs().expect("signs")[0],
        Some(Sign::Git(GitSign::Untracked))
    );
    press_key(&mut app, KeyCode::Char('i'));
    assert_eq!(
        app.view_signs().expect("signs")[0],
        Some(Sign::Git(GitSign::Untracked))
    );
}

fn press_key(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
        .expect("key");
}

#[test]
fn diagnostics_take_priority_over_git_signs() {
    let git = Some(Sign::Git(GitSign::Change));
    let diagnostic = Some(UiDiagnosticSeverity::Error);
    assert_eq!(
        preferred_sign(git.clone(), diagnostic),
        Some(Sign::Diagnostic(UiDiagnosticSeverity::Error))
    );
    assert_eq!(preferred_sign(git.clone(), None), git);
}

#[test]
fn plugin_signs_show_and_outrank_git_signs() {
    let mut app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    app.handle_resize(80, 24).expect("resize");
    app.signcolumn = SignColumnMode::Auto;
    let buffer = app.editor.buffer_number();
    let snapshot = app.editor.document_snapshot();
    app.plugin_decorations
        .set(
            "visual",
            buffer,
            DecorationScope::All,
            &[Decoration::Sign {
                line: 0,
                text: "!".to_string(),
                group: None,
                style: Some(DecorationStyle::Error),
            }],
            &snapshot,
            &app.plugin_styles,
        )
        .unwrap();
    app.update_signcolumn_width();
    let signs = app.view_signs().expect("signs");
    assert_eq!(
        signs[0],
        Some(Sign::Plugin(PluginSign {
            text: "!".to_string(),
            style: app
                .plugin_styles
                .resolve("visual", &DecorationRef::Style(DecorationStyle::Error),)
                .expect("style"),
        }))
    );
}

fn app_style(style: DecorationStyle) -> kawi_tui::frame::PluginStyle {
    let app = App::new(Buffer::from_string("alpha\n"), Flavor::Vim);
    app.plugin_styles
        .resolve("visual", &DecorationRef::Style(style))
        .expect("style")
}

#[test]
fn diagnostics_outrank_plugin_signs() {
    let plugin = Some(Sign::Plugin(PluginSign {
        text: "!".to_string(),
        style: app_style(DecorationStyle::Error),
    }));
    assert_eq!(
        preferred_sign(plugin.clone(), Some(UiDiagnosticSeverity::Warning)),
        Some(Sign::Diagnostic(UiDiagnosticSeverity::Warning))
    );
    assert_eq!(preferred_sign(plugin.clone(), None), plugin);
}

#[test]
fn auto_shows_debug_breakpoints() {
    let path = std::env::temp_dir().join(format!(
        "kawi_debug_sign_{}_{}.rs",
        std::process::id(),
        "breakpoint"
    ));
    fs::write(&path, "one\ntwo\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.handle_resize(80, 24).expect("resize");
    app.signcolumn = SignColumnMode::Auto;
    assert!(app.debug.toggle_breakpoint(&app.editor));
    app.update_signcolumn_width();
    let signs = app.view_signs().expect("signs");
    assert_eq!(signs[0], Some(Sign::Debug(DebugSign::Unverified)));
    let _ = fs::remove_file(path);
}
