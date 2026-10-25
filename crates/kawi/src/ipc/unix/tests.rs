use super::*;
use std::sync::mpsc::channel;
use std::time::Instant;

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kawi_ipc_{}_{}_{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos(),
        name
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn server_and_client_handshake() {
    let root = temp_root("handshake");
    let file = root.join("COMMIT_EDITMSG");
    fs::write(&file, "message\n").unwrap();
    let (wake, _wake_rx) = channel();
    let server = Server::start(&root, wake).unwrap();
    let socket = server.path().to_path_buf();
    let client_file = file.clone();
    let client = thread::spawn(move || wait_on(&socket, &client_file));
    let deadline = Instant::now() + Duration::from_secs(2);
    let event = loop {
        if let Some(event) = server.try_recv() {
            break event;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    };
    match event {
        Event::OpenWait { path, connection } => {
            assert_eq!(path, file);
            connection.reply(WaitOutcome::Saved).unwrap();
        }
    }
    assert_eq!(client.join().unwrap().unwrap(), WaitOutcome::Saved);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn instances_have_distinct_sockets_and_private_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root("instances");
    let (wake, _wake_rx) = channel();
    let first = Server::start(&root, wake.clone()).unwrap();
    let second = Server::start(&root, wake).unwrap();
    assert_ne!(first.path(), second.path());
    let mode = fs::metadata(first.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let parent = first.path().parent().unwrap();
    let mode = fs::metadata(parent).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_socket_fails_fast() {
    let root = temp_root("stale");
    let path = root.join("stale.sock");
    let listener = UnixListener::bind(&path).unwrap();
    drop(listener);
    let error = wait_on(&path, &root.join("message")).unwrap_err();
    assert!(error.to_string().contains("connect"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn missing_socket_fails_fast() {
    let root = temp_root("missing");
    let error = wait_on(&root.join("missing.sock"), &root.join("message")).unwrap_err();
    assert!(error.to_string().contains("connect"));
    let _ = fs::remove_dir_all(root);
}
