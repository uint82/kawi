use super::*;
use crate::actions::Action;
use crate::command::{parse, CommandCompletion};
use kawi_buffer::Buffer;
use std::collections::BTreeMap;

fn completion() -> CommandCompletion {
    CommandCompletion::new(
        BTreeMap::from([
            (
                "signcolumn".to_string(),
                vec![
                    "never".to_string(),
                    "auto".to_string(),
                    "always".to_string(),
                ],
            ),
            (
                "theme".to_string(),
                vec!["default".to_string(), "gruvbox-dark".to_string()],
            ),
        ]),
        vec![
            "auto_completion".to_string(),
            "auto_pairs".to_string(),
            "autoindent".to_string(),
            "number".to_string(),
        ],
        vec![
            "commands".to_string(),
            "configuration".to_string(),
            "option".to_string(),
            "options".to_string(),
        ],
    )
}

fn editor_with(text: &str) -> Editor {
    let mut editor = Editor::new(Buffer::from_string(text));
    editor.set_command_completion(completion());
    editor
}

fn type_command(editor: &mut Editor, text: &str) {
    editor.apply(Action::EnterCommand);
    for ch in text.chars() {
        editor.apply(Action::PushCommandChar(ch));
    }
}

fn complete(editor: &mut Editor, backward: bool) {
    editor.complete_command(backward);
}

#[test]
fn completes_command_names() {
    let mut e = editor_with("x");
    type_command(&mut e, "ed");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "edit");
    assert_eq!(
        parse(e.command_text()),
        crate::command::Command::Edit {
            name: None,
            force: false
        }
    );
}

#[test]
fn unique_command_completion_stays() {
    let mut e = editor_with("x");
    type_command(&mut e, "edi");
    complete(&mut e, false);
    complete(&mut e, false);
    assert_eq!(e.command_text(), "edit");
}

#[test]
fn cycles_commands_and_restores_original() {
    let mut e = editor_with("x");
    type_command(&mut e, "qu");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quickfix");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quit");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quit!");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quitall");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quitall!");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "qu");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quickfix");
}

#[test]
fn backward_completion_from_original_selects_last() {
    let mut e = editor_with("x");
    type_command(&mut e, "qu");
    complete(&mut e, true);
    assert_eq!(e.command_text(), "quitall!");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "qu");
}

#[test]
fn backward_from_first_returns_original() {
    let mut e = editor_with("x");
    type_command(&mut e, "qu");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "quickfix");
    complete(&mut e, true);
    assert_eq!(e.command_text(), "qu");
}

#[test]
fn completes_option_names() {
    let mut e = editor_with("x");
    type_command(&mut e, "set nu");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set number");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set numberwidth");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set nu");
}

#[test]
fn completes_no_option_names() {
    let mut e = editor_with("x");
    type_command(&mut e, "set no");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set noauto_completion");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set noauto_pairs");
}

#[test]
fn completes_option_values_after_space() {
    let mut e = editor_with("x");
    type_command(&mut e, "set theme ");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set theme default");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set theme gruvbox-dark");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set theme ");
}

#[test]
fn completes_option_values_after_assignment() {
    let mut e = editor_with("x");
    type_command(&mut e, "set theme=gr");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set theme=gruvbox-dark");
}

#[test]
fn completes_empty_assignment_values() {
    let mut e = editor_with("x");
    type_command(&mut e, "set theme=");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set theme=default");
}

#[test]
fn completes_enum_option_values() {
    let mut e = editor_with("x");
    type_command(&mut e, "set signcolumn ");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set signcolumn always");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set signcolumn auto");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set signcolumn never");
}

#[test]
fn completes_help_topics() {
    let mut e = editor_with("x");
    type_command(&mut e, "help conf");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "help configuration");
}

#[test]
fn completes_help_option_names() {
    let mut e = editor_with("x");
    type_command(&mut e, "help option th");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "help option theme");
}

#[test]
fn option_completion_does_not_cross_prefix() {
    let mut e = editor_with("x");
    type_command(&mut e, "set nu");
    complete(&mut e, false);
    complete(&mut e, false);
    assert_eq!(e.command_text(), "set numberwidth");
}

#[test]
fn typing_clears_completion() {
    let mut e = editor_with("x");
    type_command(&mut e, "q");
    complete(&mut e, false);
    assert_eq!(e.command_text(), "q");
    e.apply(Action::PushCommandChar('!'));
    assert_eq!(e.completion, None);
}

#[test]
fn cancel_clears_completion() {
    let mut e = editor_with("x");
    type_command(&mut e, "q");
    complete(&mut e, false);
    e.apply(Action::CancelCommand);
    assert_eq!(e.completion, None);
}

#[test]
fn completes_buffer_names() {
    let mut e = editor_with("x");
    e.edit_file(Some("alpha.txt"), false).unwrap();
    e.edit_file(Some("beta.txt"), false).unwrap();
    type_command(&mut e, "b a");
    complete(&mut e, false);
    assert!(e.command_text().contains("alpha.txt") || e.command_text().contains("beta.txt"));
}

#[test]
fn file_candidates_match_prefix_and_directories() {
    let dir = std::env::temp_dir().join("kawi_completion_files");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("alpha.txt"), "x").unwrap();
    fs::write(dir.join("alfred.txt"), "x").unwrap();
    fs::write(dir.join("beta.rs"), "x").unwrap();
    fs::write(dir.join(".hidden"), "x").unwrap();
    fs::write(dir.join("src/main.rs"), "x").unwrap();
    let home = Path::new("/nonexistent");
    assert_eq!(
        file_candidates("al", &dir, home),
        vec!["alfred.txt".to_string(), "alpha.txt".to_string()]
    );
    assert_eq!(file_candidates("sr", &dir, home), vec!["src/".to_string()]);
    assert_eq!(
        file_candidates("src/", &dir, home),
        vec!["src/main.rs".to_string()]
    );
    assert_eq!(
        file_candidates(".h", &dir, home),
        vec![".hidden".to_string()]
    );
    assert!(file_candidates("", &dir, home)
        .iter()
        .all(|name| !name.starts_with('.')));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn completing_directory_restarts_inside() {
    let dir = std::env::temp_dir().join("kawi_completion_dirs");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "x").unwrap();
    fs::write(dir.join("src/lib.rs"), "x").unwrap();
    assert_eq!(
        file_candidates("sr", &dir, Path::new("/nonexistent")),
        vec!["src/".to_string()]
    );
    assert_eq!(
        file_candidates("src/", &dir, Path::new("/nonexistent")),
        vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_candidates_use_home_for_tilde() {
    let home = std::env::temp_dir().join("kawi_completion_home");
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("homely.txt"), "x").unwrap();
    let candidates = file_candidates("~/hom", Path::new("/nonexistent"), &home);
    assert_eq!(candidates, vec!["~/homely.txt".to_string()]);
    let _ = fs::remove_dir_all(&home);
}
