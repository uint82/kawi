use super::*;

#[test]
fn log_picker_creates_a_branch_at_the_selected_commit() {
    let dir = repo("log_branch");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    app.handle_picker_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.branch_prompt.is_some());
    for character in "at-commit".chars() {
        app.handle_branch_prompt_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
            .unwrap();
    }
    app.handle_branch_prompt_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(git(&dir, &["branch", "--list", "at-commit"]).contains("at-commit"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn log_picker_streams_and_opens_readonly_diffs() {
    let dir = repo("log");
    let path = dir.join("a.txt");
    let root = commit(&dir, "a.txt", "one\n");
    commit(&dir, "a.txt", "two\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    assert_eq!(
        app.picker.as_ref().unwrap().picker().kind(),
        PickerKind::GitLog
    );
    app.handle_picker_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.editor.current_buffer_is_readonly());
    assert!(app
        .editor
        .text_lines()
        .iter()
        .any(|line| line.contains("+two")));
    let before = app.editor.text_lines();
    app.editor.apply(Action::EnterInsert);
    app.editor.apply(Action::InsertChar('x'));
    app.editor.apply(Action::ExitInsert);
    assert_eq!(app.editor.text_lines(), before);
    assert!(app
        .open_files()
        .iter()
        .all(|(number, _)| *number != app.editor.buffer_number()));
    let _ = root;
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn log_picker_loads_lazily_as_the_selection_moves() {
    let dir = repo("lazy");
    let path = dir.join("a.txt");
    for index in 0..140 {
        commit(&dir, "a.txt", &format!("commit {index}"));
    }
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    let loaded = wait_log(&mut app);
    assert!(loaded < 140);
    let session = app.picker.as_mut().unwrap();
    session.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    let loaded_after = wait_log_growth(&mut app, loaded);
    assert!(loaded_after > loaded);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn log_picker_opens_commit_files_from_the_keymap() {
    let dir = repo("log_keys");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    commit(&dir, "a.txt", "two\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    app.handle_picker_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(
        app.picker.as_ref().unwrap().picker().kind(),
        PickerKind::GitCommitFiles
    );
    app.handle_picker_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.editor.current_buffer_is_readonly());
    assert!(app
        .editor
        .text_lines()
        .iter()
        .any(|line| line.contains("+two")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn commit_files_open_a_readonly_file_diff() {
    let dir = repo("file_diff");
    let path = dir.join("a.txt");
    let root = commit(&dir, "a.txt", "one\n");
    commit(&dir, "a.txt", "two\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    let sha = app.picker.as_ref().unwrap().picker().rows()[0].path.clone();
    app.open_commit_files(&dir, &sha).unwrap();
    assert_eq!(
        app.picker.as_ref().unwrap().picker().kind(),
        PickerKind::GitCommitFiles
    );
    app.handle_picker_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.editor.current_buffer_is_readonly());
    assert!(app
        .editor
        .text_lines()
        .iter()
        .any(|line| line.contains("+two")));
    let _ = root;
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn log_picker_checks_out_the_selected_commit_detached() {
    let dir = repo("log_checkout");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    app.handle_picker_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(app.git_branch.pending_checkout.is_some());
    app.handle_pending_branch_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn log_picker_keeps_open_when_copying_a_sha() {
    let dir = repo("log_copy");
    let path = dir.join("a.txt");
    commit(&dir, "a.txt", "one\n");
    let mut app = app(&path);
    app.open_log_picker(false, None, None).unwrap();
    wait_log(&mut app);
    app.handle_picker_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(
        app.picker.as_ref().unwrap().picker().kind(),
        PickerKind::GitLog
    );
    let _ = fs::remove_dir_all(dir);
}
