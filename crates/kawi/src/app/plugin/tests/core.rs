use std::fs;
use std::time::{Duration, Instant};

use kawi_buffer::Buffer;
use kawi_editor::Action;
use kawi_plugin::{
    ApplyEditParams, DiagnosticSummary, GetStateParams, Position, ProtocolTextEdit,
    SetDiagnosticsParams, INVALID_PARAMS, INVALID_POSITION, STALE_REVISION,
};
use serde_json::json;

use super::common::*;
use crate::app::App;
use crate::command::{CommandHandler, CommandSource};
use crate::keymap::{BindingSource, Flavor};
use kawi_plugin::PluginStatus;

#[test]
fn static_contributions_register_commands_and_bindings() {
    let root = temp_dir("static");
    manifest(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("echo.ping").expect("plugin command");
    let spec = app.commands.spec(command).expect("spec");
    assert_eq!(spec.handler, CommandHandler::Plugin(0));
    assert_eq!(spec.source, CommandSource::Plugin("echo".to_string()));
    assert_eq!(app.plugins.target(0), Some(("echo", "ping")));
    assert_eq!(app.keymaps.bindings().len(), 2);
    assert!(app
        .keymaps
        .bindings()
        .iter()
        .all(|binding| binding.source == BindingSource::Plugin));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn keymap_reload_keeps_plugin_defaults() {
    let root = temp_dir("reload");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.apply_keymaps(crate::keymap::config::KeymapLoad::default());
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nunmap_does_not_remove_plugin_defaults() {
    let root = temp_dir("nunmap");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.handle_map_command("nunmap <leader>pp").unwrap();
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lists_plugins_without_starting_them() {
    let root = temp_dir("list");
    manifest(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    app.handle_plugin_command("list");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines[0].contains("echo"));
    assert!(lines[0].contains("stopped"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn get_state_uses_utf8_byte_columns() {
    let mut app = App::new(Buffer::from_string("héllo"), Flavor::Vim);
    app.editor.apply(Action::MoveRight);
    app.editor.apply(Action::MoveRight);
    let params = serde_json::to_value(GetStateParams { include_text: true }).unwrap();
    let value = app
        .handle_plugin_request("test", "editor.get_state", params)
        .unwrap();
    assert_eq!(value["text"], "héllo");
    assert_eq!(value["cursor"]["line"], 0);
    assert_eq!(value["cursor"]["byte_column"], 3);
    assert_eq!(value["mode"], "normal");
}

#[test]
fn applies_edits_at_the_current_revision() {
    let mut app = App::new(Buffer::from_string("héllo"), Flavor::Vim);
    let revision = app.editor.buffer_revision();
    let params = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64,
        revision,
        edits: vec![ProtocolTextEdit {
            start: Position {
                line: 0,
                byte_column: 1,
            },
            end: Position {
                line: 0,
                byte_column: 3,
            },
            text: "i".to_string(),
        }],
    };
    let value = app
        .handle_plugin_request(
            "test",
            "editor.apply_edit",
            serde_json::to_value(params).unwrap(),
        )
        .unwrap();
    assert_eq!(app.editor.text_lines(), vec!["hillo"]);
    assert_eq!(value["revision"], revision + 1);
}

#[test]
fn rejects_stale_revisions_and_split_code_points() {
    let mut app = app();
    let stale = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64,
        revision: 99,
        edits: Vec::new(),
    };
    let error = app
        .handle_plugin_request(
            "test",
            "editor.apply_edit",
            serde_json::to_value(stale).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    assert_eq!(app.editor.text_lines(), vec!["alpha"]);

    let wrong_buffer = ApplyEditParams {
        buffer: app.editor.buffer_number() as u64 + 100,
        revision: app.editor.buffer_revision(),
        edits: Vec::new(),
    };
    let error = app
        .handle_plugin_request(
            "test",
            "editor.apply_edit",
            serde_json::to_value(wrong_buffer).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    assert!(error.message.contains("stale buffer"));

    for text in ["é", "中", "😀"] {
        let mut app = App::new(Buffer::from_string(text), Flavor::Vim);
        let revision = app.editor.buffer_revision();
        let split = ApplyEditParams {
            buffer: app.editor.buffer_number() as u64,
            revision,
            edits: vec![ProtocolTextEdit {
                start: Position {
                    line: 0,
                    byte_column: 1,
                },
                end: Position {
                    line: 0,
                    byte_column: 2,
                },
                text: "x".to_string(),
            }],
        };
        let error = app
            .handle_plugin_request(
                "test",
                "editor.apply_edit",
                serde_json::to_value(split).unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.code, INVALID_POSITION, "{text}");
        assert_eq!(app.editor.text_lines(), vec![text], "{text}");
    }
}

#[test]
fn broken_manifest_warns_and_keeps_good_plugins() {
    let root = temp_dir("bad_manifest");
    manifest(&root);
    let broken = root.join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("plugin.toml"), "name = 7\nprotocol = true\n").unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.iter().any(|warning| warning.contains("broken")));
    assert!(app.commands.find("echo.ping").is_some());
    assert_eq!(app.keymaps.bindings().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn returns_method_not_found_for_unknown_host_methods() {
    let mut app = app();
    let error = app
        .handle_plugin_request("test", "editor.unknown", json!({}))
        .unwrap_err();
    assert_eq!(error.code, kawi_plugin::METHOD_NOT_FOUND);
}

#[test]
fn plugin_context_carries_the_current_cursor() {
    let app = App::new(Buffer::from_string("abé"), Flavor::Vim);
    let context = app.plugin_context();
    assert_eq!(context.cursor.line, 0);
    assert_eq!(context.cursor.byte_column, 0);
    assert_eq!(context.mode, "normal");
}

#[test]
fn invokes_a_plugin_and_serves_host_requests() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("invoke");
    let dir = root.join("echo");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/echo");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("echo.hello").expect("hello command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.message.as_deref() != Some("pong world") && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.message.as_deref(), Some("pong world"));
    assert_eq!(app.plugins.status("echo"), Some(PluginStatus::Ready));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn trim_plugin_applies_a_real_edit() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("trim");
    let dir = root.join("trim");
    fs::create_dir_all(&dir).unwrap();
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/trim");
    fs::copy(source.join("plugin.toml"), dir.join("plugin.toml")).unwrap();
    fs::copy(source.join("plugin.py"), dir.join("plugin.py")).unwrap();
    let mut app = App::new(Buffer::from_string("alpha  \nbeta\t\n"), Flavor::Vim);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let command = app.commands.find("trim.trim").expect("trim command");
    let target = match app.commands.spec(command).expect("spec").handler {
        CommandHandler::Plugin(target) => target,
        other => panic!("unexpected handler: {other:?}"),
    };
    app.run_plugin_command(target, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.editor.document_snapshot() != "alpha\nbeta\n" && Instant::now() < deadline {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.editor.document_snapshot().to_string(), "alpha\nbeta\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn publishes_scoped_diagnostics_for_the_current_revision() {
    let dir = temp_dir("diagnostics");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\nbeta\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    let revision = app.editor.buffer_revision();
    let params = SetDiagnosticsParams {
        buffer: app.editor.buffer_number() as u64,
        revision,
        diagnostics: vec![DiagnosticSummary {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 5,
            },
            severity: "warning".to_string(),
            message: "check this".to_string(),
            source: Some("spoofed".to_string()),
            code: Some("W1".to_string()),
        }],
    };
    app.handle_plugin_request(
        "lint",
        "editor.set_diagnostics",
        serde_json::to_value(params).unwrap(),
    )
    .unwrap();
    let published = app.problems.problems_for_path(&path);
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].source, "plugin:lint");
    assert_eq!(published[0].provider.as_deref(), Some("lint"));
    assert_eq!(published[0].message, "check this");

    app.handle_plugin_request(
        "lint",
        "editor.set_diagnostics",
        json!({
            "buffer": app.editor.buffer_number(),
            "revision": app.editor.buffer_revision(),
            "diagnostics": []
        }),
    )
    .unwrap();
    assert!(app.problems.problems_for_path(&path).is_empty());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_unknown_diagnostic_severities_and_stale_revisions() {
    let dir = temp_dir("diagnostics_invalid");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    let params = json!({
        "buffer": app.editor.buffer_number(),
        "revision": app.editor.buffer_revision(),
        "diagnostics": [{
            "start": {"line": 0, "byte_column": 0},
            "end": {"line": 0, "byte_column": 1},
            "severity": "fatal",
            "message": "bad",
            "source": null,
            "code": null
        }]
    });
    let error = app
        .handle_plugin_request("lint", "editor.set_diagnostics", params.clone())
        .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);

    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            json!({
                "buffer": app.editor.buffer_number(),
                "revision": 999,
                "diagnostics": []
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, STALE_REVISION);
    fs::remove_dir_all(dir).unwrap();
}
