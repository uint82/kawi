use super::*;
use crate::keymap::Flavor;
use kawi_buffer::Buffer;
use kawi_editor::BufferOptionDefaults;
use kawi_syntax::LanguageId;
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_language_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn app(path: &str) -> App {
    let mut app = plain_app(path);
    app.editor.set_buffer_defaults(BufferOptionDefaults {
        shiftwidth: Some(6),
        expandtab: Some(false),
        autoindent: true,
    });
    app
}

fn plain_app(path: &str) -> App {
    let mut buffer = Buffer::from_string("fn main() {}\n");
    buffer.path = Some(path.to_string());
    App::new(buffer, Flavor::Vim)
}

fn load_languages(app: &mut App, dir: &Path, text: &str) {
    let path = dir.join("languages.toml");
    fs::write(&path, text).unwrap();
    app.lsp.load(&path, app.events.wake());
    app.sync_language_settings();
}

#[test]
fn language_editing_uses_configured_tokens_and_pairs() {
    let dir = temp_dir("editing");
    let mut app = app("main.rs");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nline_comment = \"#\"\nblock_comment = []\nauto_pairs = [[\"<\", \">\"]]\nmatch_pairs = [[\"(\", \")\"], [\"<\", \">\"]]\nauto_close_before = \"?,\"\n",
    );
    app.sync_language_editing();
    let editing = app.editor.language_editing();
    assert_eq!(editing.comments.line.as_deref(), Some("#"));
    assert_eq!(editing.comments.block, None);
    assert_eq!(editing.pairs, vec![kawi_editor::Pair::new('<', '>')]);
    assert_eq!(
        editing.match_pairs,
        vec![
            kawi_editor::Pair::new('(', ')'),
            kawi_editor::Pair::new('<', '>'),
        ]
    );
    assert_eq!(editing.auto_close_before, "?,");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn language_editing_can_disable_auto_close_punctuation() {
    let dir = temp_dir("editing_auto_close_empty");
    let mut app = app("main.rs");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nauto_close_before = \"\"\n",
    );
    let editing = app.editor.language_editing();
    assert_eq!(editing.auto_close_before, "");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn built_in_language_editing_is_available() {
    let dir = temp_dir("editing_builtin");
    let mut app = app("main.rs");
    load_languages(&mut app, &dir, "");
    app.sync_language_editing();
    let editing = app.editor.language_editing();
    assert_eq!(editing.comments.line.as_deref(), Some("//"));
    assert!(editing.pairs.contains(&kawi_editor::Pair::new('(', ')')));
    assert!(!editing.pairs.contains(&kawi_editor::Pair::new('\'', '\'')));
    assert_eq!(
        editing.match_pairs,
        vec![
            kawi_editor::Pair::new('(', ')'),
            kawi_editor::Pair::new('[', ']'),
            kawi_editor::Pair::new('{', '}'),
        ]
    );
    assert_eq!(
        editing.auto_close_before,
        kawi_editor::DEFAULT_AUTO_CLOSE_BEFORE
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn built_in_ada_matches_brackets_across_attribute_ticks() {
    let dir = temp_dir("editing_ada");
    let mut buffer = Buffer::from_string("Put_Line (\"cool \" & Color'Image (C));\n");
    buffer.path = Some("main.adb".to_string());
    let mut app = App::new(buffer, Flavor::Vim);
    load_languages(&mut app, &dir, "");
    app.sync_language_editing();
    let editing = app.editor.language_editing();
    assert_eq!(editing.comments.line.as_deref(), Some("--"));
    assert!(!editing.pairs.contains(&kawi_editor::Pair::new('\'', '\'')));
    app.editor.apply(kawi_editor::Action::JumpTo(0, 9));
    assert_eq!(
        app.editor
            .bracket_match(kawi_editor::HIGHLIGHT_MATCH_STEP_BUDGET),
        Some(kawi_editor::BracketMatch {
            first: (0, 9),
            second: (0, 35),
        })
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn built_in_adl_editing_uses_adl_tokens() {
    let dir = temp_dir("editing_adl");
    let mut buffer =
        Buffer::from_string("module demo {\n  struct User<T> {\n    T value;\n  };\n};\n");
    buffer.path = Some("model.adl-java".to_string());
    let mut app = App::new(buffer, Flavor::Vim);
    load_languages(&mut app, &dir, "");
    app.sync_language_editing();
    let editing = app.editor.language_editing();
    assert_eq!(editing.comments.line.as_deref(), Some("//"));
    assert_eq!(editing.comments.block, None);
    assert_eq!(
        editing.pairs,
        vec![
            kawi_editor::Pair::new('[', ']'),
            kawi_editor::Pair::new('{', '}'),
            kawi_editor::Pair::new('<', '>'),
            kawi_editor::Pair::new('"', '"'),
        ]
    );
    assert_eq!(app.editor.shiftwidth(), 4);
    assert!(app.editor.expandtab());
    assert_eq!(app.editor.tabstop(), 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn built_in_language_indent_applies() {
    let dir = temp_dir("builtin_indent");
    let mut app = plain_app("main.py");
    load_languages(&mut app, &dir, "");
    assert_eq!(app.editor.shiftwidth(), 4);
    assert!(app.editor.expandtab());
    assert_eq!(app.editor.tabstop(), 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn editor_defaults_precede_built_in_language_indent() {
    let dir = temp_dir("global_indent");
    let mut app = app("main.py");
    load_languages(&mut app, &dir, "");
    assert_eq!(app.editor.shiftwidth(), 6);
    assert!(!app.editor.expandtab());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn save_as_syncs_language_indent() {
    let dir = temp_dir("save_as_indent");
    let source = dir.join("a.txt");
    fs::write(&source, "alpha\n").unwrap();
    let mut app = plain_app(source.to_str().unwrap());
    load_languages(&mut app, &dir, "");
    assert_eq!(app.editor.shiftwidth(), 8);
    let target = dir.join("copy.rs");
    app.run_command(kawi_editor::parse(&format!("saveas {}", target.display())).expect("parse"))
        .unwrap();
    assert_eq!(app.editor.shiftwidth(), 4);
    assert!(app.editor.expandtab());
    assert_eq!(app.editor.tabstop(), 4);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn python_indent_rules_are_loaded() {
    let dir = temp_dir("python_rules");
    let mut app = app("main.py");
    load_languages(&mut app, &dir, "");
    let indent = app.lsp.language_indent(Some(Path::new("main.py")));
    assert_eq!(indent.language, Some(LanguageId::Python));
    assert_eq!(indent.rules.indent_after_suffixes, vec![":"]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn html_indent_rules_are_loaded() {
    let dir = temp_dir("html_rules");
    let mut app = app("index.html");
    load_languages(&mut app, &dir, "");
    let indent = app.lsp.language_indent(Some(Path::new("index.html")));
    assert_eq!(indent.language, Some(LanguageId::Html));
    assert!(indent.rules.indent_tags);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sql_indent_rules_are_loaded() {
    let dir = temp_dir("sql_rules");
    let mut app = app("query.sql");
    load_languages(&mut app, &dir, "");
    let indent = app.lsp.language_indent(Some(Path::new("query.sql")));
    assert_eq!(indent.language, Some(LanguageId::Sql));
    assert_eq!(indent.defaults.shiftwidth, Some(2));
    assert!(indent.rules.indent_ignore_case);
    assert!(indent.rules.indent_keywords_in_brackets);
    assert_eq!(indent.rules.indent_block_closers, vec!["end"]);
    assert!(indent
        .rules
        .indent_align_keywords
        .contains(&"from".to_string()));
    assert!(indent
        .rules
        .indent_branch_keywords
        .contains(&"when".to_string()));
    assert!(indent
        .rules
        .indent_branch_openers
        .contains(&"case".to_string()));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn language_indent_overrides_are_loaded() {
    let dir = temp_dir("indent_overrides");
    let mut app = app("main.rs");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nindent_after_suffixes = [\"=>\"]\nindent_after_keywords = [\"loop\"]\nindent_brackets = false\n",
    );
    let indent = app.lsp.language_indent(Some(Path::new("main.rs")));
    assert_eq!(indent.rules.indent_after_suffixes, vec!["=>"]);
    assert_eq!(indent.rules.indent_after_keywords, vec!["loop"]);
    assert!(!indent.rules.indent_brackets);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn language_indent_precedes_editor_defaults() {
    let dir = temp_dir("override");
    let mut app = app("main.rs");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
    );
    assert_eq!(app.editor.shiftwidth(), 2);
    assert!(app.editor.expandtab());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn language_indent_travels_with_each_buffer() {
    let dir = temp_dir("buffers");
    let mut app = app("main.rs");
    let first = app.editor.buffer_number();
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
    );
    app.editor.open_empty_buffer();
    app.sync_language_indent();
    assert_eq!(app.editor.shiftwidth(), 6);
    assert!(!app.editor.expandtab());
    assert!(app.editor.buffer_select(first));
    app.sync_language_indent();
    assert_eq!(app.editor.shiftwidth(), 2);
    assert!(app.editor.expandtab());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn runtime_indent_survives_language_sync() {
    let dir = temp_dir("runtime");
    let mut app = app("main.rs");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
    );
    app.run_command(kawi_editor::parse("set sw=5").expect("parse"))
        .unwrap();
    app.run_command(kawi_editor::parse("set noexpandtab").expect("parse"))
        .unwrap();
    app.sync_language_indent();
    assert_eq!(app.editor.shiftwidth(), 5);
    assert!(!app.editor.expandtab());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn buffers_without_a_language_use_editor_defaults() {
    let dir = temp_dir("fallback");
    let mut app = app("notes.txt");
    load_languages(
        &mut app,
        &dir,
        "[[language]]\nname = \"rust\"\nshiftwidth = 2\nexpandtab = true\n",
    );
    assert_eq!(app.editor.shiftwidth(), 6);
    assert!(!app.editor.expandtab());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cpp_closing_brace_keeps_the_current_line_indent() {
    let dir = temp_dir("cpp_close");
    let mut buffer = Buffer::from_string(
        "namespace demo {\nconcept Addable = requires(T value) {\n    value + value;\n};\n}",
    );
    buffer.path = Some("test.cpp".to_string());
    let mut app = App::new(buffer, Flavor::Vim);
    load_languages(&mut app, &dir, "");
    app.editor.apply(kawi_editor::Action::JumpTo(3, 0));
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('o'),
        crossterm::event::KeyModifiers::NONE,
    ))
    .unwrap();
    assert_eq!(app.editor.text_lines()[4], "");
    let _ = fs::remove_dir_all(dir);
}
