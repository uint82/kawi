use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use kawi_plugin::{
    load_catalog, CapabilityRegistry, HostEnvironment, InvocationContext, PluginHost,
    PluginHostEvent, PluginStatus, Position,
};
use serde_json::{json, Value};

use super::scaffold::{self, Options};

const START_TIMEOUT: Duration = Duration::from_secs(60);
const INVOCATION_TIMEOUT: Duration = Duration::from_secs(30);
const STATUS_POLL: Duration = Duration::from_millis(10);
const PLUGIN_NAME: &str = "scaffold-demo";
const COMMAND: &str = "hello";

#[test]
#[ignore = "builds a nested cargo project"]
fn scaffold_builds_and_runs() {
    let root = temp_root();
    let plugins = root.join("plugins");
    let config = root.join("config");
    fs::create_dir_all(&plugins).unwrap();
    fs::create_dir_all(&config).unwrap();
    let directory = plugins.join(PLUGIN_NAME);
    let sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../kawi-plugin-sdk");
    let options = Options {
        name: PLUGIN_NAME.to_string(),
        sdk_path: Some(sdk),
    };
    scaffold::create_project(&directory, &options).unwrap();
    super::build::run(&[directory.to_string_lossy().to_string()]).unwrap();

    let catalog = load_catalog(&plugins);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let (event_tx, event_rx) = mpsc::channel();
    let (wake, wake_rx) = mpsc::channel();
    let (finished_tx, finished_rx) = mpsc::channel();
    let drain = thread::spawn(move || {
        for event in event_rx {
            if let PluginHostEvent::InvocationFinished {
                request, result, ..
            } = event
            {
                let _ = finished_tx.send((request, result));
            }
        }
    });
    let environment = HostEnvironment::from_paths(Some(root.clone()), Some(config));
    let host = PluginHost::start(
        &catalog,
        CapabilityRegistry::default(),
        environment,
        event_tx,
        wake,
    );
    host.activate(PLUGIN_NAME).unwrap();
    wait_ready(&host);
    let request = host
        .invoke(PLUGIN_NAME, COMMAND, Value::Null, context())
        .unwrap();
    let (finished, result) = finished_rx.recv_timeout(INVOCATION_TIMEOUT).unwrap();
    assert_eq!(finished, request);
    assert_eq!(result.unwrap(), json!(format!("hello from {PLUGIN_NAME}")));
    host.shutdown();
    drop(host);
    drop(wake_rx);
    drain.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn wait_ready(host: &PluginHost) {
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        match host.status(PLUGIN_NAME) {
            Some(PluginStatus::Ready) => return,
            Some(PluginStatus::Disabled) | Some(PluginStatus::Error(_)) => {
                panic!("scaffold did not start: {:?}", host.status(PLUGIN_NAME))
            }
            _ if Instant::now() >= deadline => panic!("scaffold did not become ready"),
            _ => thread::sleep(STATUS_POLL),
        }
    }
}

fn context() -> InvocationContext {
    InvocationContext {
        buffer: 1,
        revision: 1,
        path: None,
        language: None,
        mode: "normal".to_string(),
        cursor: Position {
            line: 0,
            byte_column: 0,
        },
        selection: None,
    }
}

fn temp_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("kawi-plugin-cli-{}-{nanos}", std::process::id()))
}
