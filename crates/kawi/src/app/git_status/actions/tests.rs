use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;

use crate::app::App;
use crate::keymap::Flavor;

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn app_with_repo(name: &str) -> (std::path::PathBuf, App) {
    let dir = crate::test_support::temp_dir(name);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    let path = dir.join("a.txt");
    fs::write(&path, "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-qm", "one"]);
    fs::write(&path, "two\n").unwrap();
    let app = App::new(
        Buffer::from_file(path.to_str().unwrap()).unwrap(),
        Flavor::Vim,
    );
    (dir, app)
}

fn wait_for_status(app: &mut App) {
    app.refresh_git_status();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.git_status.status().is_none() {
        assert!(Instant::now() < deadline, "git status did not load");
        app.sync_git_status();
        thread::sleep(Duration::from_millis(5));
    }
}

fn key(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
}

#[test]
fn stages_unstages_and_discards_the_selected_entry() {
    let (dir, mut app) = app_with_repo("actions");
    wait_for_status(&mut app);
    app.open_git_status();
    assert!(app.git_status.has_view());
    app.handle_git_status_key(key('s')).unwrap();
    wait_for_status(&mut app);
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "a.txt");
    app.handle_git_status_key(key('u')).unwrap();
    wait_for_status(&mut app);
    assert_eq!(git(&dir, &["diff", "--cached", "--name-only"]), "");
    app.handle_git_status_key(key('x')).unwrap();
    assert!(app.last_message().unwrap().contains("Discard changes"));
    app.handle_git_status_key(key('y')).unwrap();
    wait_for_status(&mut app);
    assert_eq!(fs::read_to_string(dir.join("a.txt")).unwrap(), "one\n");
    let _ = fs::remove_dir_all(dir);
}
