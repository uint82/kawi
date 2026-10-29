use super::*;

#[test]
fn bridges_editor_state_to_a_wasm_component() {
    let plugin = request(plugin_with("editor-host.wasm"), &[Capability::EditorRead]);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let (events_tx, events_rx) = mpsc::channel();
    let (wake_tx, _wake_rx) = mpsc::channel();
    let capabilities = grants(&name, &[Capability::EditorRead]);
    let handle = start_wasm_worker(
        plugin,
        capabilities,
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    handle
        .invoke(
            8,
            "read".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut responded = false;
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::HostRequest { id, method, .. } if method == "editor.get_state" => {
                handle
                    .respond(
                        id,
                        Ok(json!({
                            "buffer": 42,
                            "revision": 3,
                            "path": null,
                            "language": null,
                            "mode": "normal",
                            "cursor": { "line": 0, "byte_column": 0 },
                            "selection": null,
                            "text": null
                        })),
                    )
                    .unwrap();
                responded = true;
            }
            PluginHostEvent::InvocationFinished { result, .. } => {
                assert!(responded);
                assert_eq!(result.unwrap(), json!(42));
                break;
            }
            PluginHostEvent::Log { line, .. } => panic!("plugin log: {line}"),
            _ => {}
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
