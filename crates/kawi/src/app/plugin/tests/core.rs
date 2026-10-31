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
    assert!(lines[0].contains("process (trusted compatibility runtime)"));
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

#[test]
fn plugin_diagnostics_are_scoped_to_their_source() {
    let dir = temp_dir("diagnostics_scope");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    publish_diagnostic(&mut app, "one", 0, 0, 0, 1);
    publish_diagnostic(&mut app, "two", 0, 0, 0, 1);
    assert_eq!(app.problems.problems_for_path(&path).len(), 2);
    clear_diagnostics(&mut app, "one");
    let remaining = app.problems.problems_for_path(&path);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].source, "plugin:two");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn diagnostics_reject_invalid_positions_and_oversized_messages() {
    let dir = temp_dir("diagnostics_limits");
    let path = dir.join("alpha.txt");
    fs::write(&path, "aé\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            json!({
                "buffer": app.editor.buffer_number(),
                "revision": app.editor.buffer_revision(),
                "diagnostics": [{
                    "start": {"line": 0, "byte_column": 2},
                    "end": {"line": 0, "byte_column": 3},
                    "severity": "error",
                    "message": "split",
                    "source": null,
                    "code": null
                }]
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_POSITION);

    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            json!({
                "buffer": app.editor.buffer_number(),
                "revision": app.editor.buffer_revision(),
                "diagnostics": [{
                    "start": {"line": 99, "byte_column": 0},
                    "end": {"line": 99, "byte_column": 1},
                    "severity": "error",
                    "message": "out of range",
                    "source": null,
                    "code": null
                }]
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_POSITION);

    let too_many = (0..1_001)
        .map(|_| {
            json!({
                "start": {"line": 0, "byte_column": 0},
                "end": {"line": 0, "byte_column": 1},
                "severity": "error",
                "message": "many",
                "source": null,
                "code": null
            })
        })
        .collect::<Vec<_>>();
    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            diagnostics_params(&app, too_many),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);

    let message = "x".repeat(4_097);
    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            json!({
                "buffer": app.editor.buffer_number(),
                "revision": app.editor.buffer_revision(),
                "diagnostics": [{
                    "start": {"line": 0, "byte_column": 0},
                    "end": {"line": 0, "byte_column": 1},
                    "severity": "error",
                    "message": message,
                    "source": null,
                    "code": null
                }]
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn diagnostic_publish_calls_are_rate_limited() {
    let dir = temp_dir("diagnostics_rate");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    for _ in 0..20 {
        assert!(clear_diagnostics(&mut app, "lint"));
    }
    let error = app
        .handle_plugin_request(
            "lint",
            "editor.set_diagnostics",
            diagnostics_params(&app, Vec::new()),
        )
        .unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn closing_a_buffer_clears_plugin_diagnostics() {
    let dir = temp_dir("diagnostics_close");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    app.poll_plugins();
    publish_diagnostic(&mut app, "lint", 0, 0, 0, 1);
    assert_eq!(app.problems.problems_for_path(&path).len(), 1);
    let number = app.editor.buffer_number();
    app.editor.close_buffer(number, true).unwrap();
    app.poll_plugins();
    assert!(app.problems.problems_for_path(&path).is_empty());
    fs::remove_dir_all(dir).unwrap();
}

fn publish_diagnostic(
    app: &mut App,
    plugin: &str,
    start_column: u64,
    end_column: u64,
    line: u64,
    end_line: u64,
) {
    let params = json!({
        "buffer": app.editor.buffer_number(),
        "revision": app.editor.buffer_revision(),
        "diagnostics": [{
            "start": {"line": line, "byte_column": start_column},
            "end": {"line": end_line, "byte_column": end_column},
            "severity": "warning",
            "message": "check",
            "source": null,
            "code": null
        }]
    });
    app.handle_plugin_request(plugin, "editor.set_diagnostics", params)
        .unwrap();
}

fn clear_diagnostics(app: &mut App, plugin: &str) -> bool {
    app.handle_plugin_request(
        plugin,
        "editor.set_diagnostics",
        diagnostics_params(app, Vec::new()),
    )
    .is_ok()
}

fn diagnostics_params(app: &App, diagnostics: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "buffer": app.editor.buffer_number(),
        "revision": app.editor.buffer_revision(),
        "diagnostics": diagnostics
    })
}

#[test]
fn stopping_a_plugin_clears_its_diagnostics() {
    let dir = temp_dir("diagnostics_stop");
    let path = dir.join("alpha.txt");
    fs::write(&path, "alpha\n").unwrap();
    let mut app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    publish_diagnostic(&mut app, "lint", 0, 0, 0, 1);
    assert_eq!(app.problems.problems_for_path(&path).len(), 1);
    for status in [
        kawi_plugin::PluginStatus::Stopped,
        kawi_plugin::PluginStatus::Disabled,
        kawi_plugin::PluginStatus::Error("crashed".to_string()),
    ] {
        publish_diagnostic(&mut app, "lint", 0, 0, 0, 1);
        app.handle_plugin_event(kawi_plugin::PluginHostEvent::StatusChanged {
            plugin: "lint".to_string(),
            status,
        });
        assert!(app.problems.problems_for_path(&path).is_empty());
    }
    fs::remove_dir_all(dir).unwrap();
}
