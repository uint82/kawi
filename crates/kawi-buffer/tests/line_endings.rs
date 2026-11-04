use std::fs;
use std::path::PathBuf;

use kawi_buffer::{Buffer, LineEnding};

fn case_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kawi_line_endings_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create case dir");
    dir
}

fn open_file(dir: &std::path::Path, name: &str, text: &str) -> Buffer {
    let path = dir.join(name);
    fs::write(&path, text).expect("seed file");
    Buffer::from_file(path.to_str().expect("utf8 path")).expect("open buffer")
}

#[test]
fn saves_edits_with_crlf() {
    let dir = case_dir("crlf_save");
    let path = dir.join("file.txt");
    let mut buffer = open_file(&dir, "file.txt", "one\r\ntwo\r\n");
    assert_eq!(buffer.line_ending(), LineEnding::Crlf);
    assert_eq!(buffer.line(0).as_deref(), Some("one"));
    buffer.insert_newline(0, 3);
    buffer.delete_backward(1, 0);
    buffer.insert_char(0, 3, '!');
    buffer.insert_newline(0, 4);
    buffer.save(false).expect("save");
    assert_eq!(
        fs::read(&path).expect("read"),
        b"one!\r\n\r\ntwo\r\n".to_vec()
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn chooses_the_majority_line_ending() {
    let dir = case_dir("mixed");
    let path = dir.join("file.txt");
    let mut buffer = open_file(&dir, "file.txt", "a\r\nb\r\nc\n");
    assert_eq!(buffer.line_ending(), LineEnding::Crlf);
    buffer.save(false).expect("save");
    assert_eq!(fs::read(&path).expect("read"), b"a\r\nb\r\nc\r\n".to_vec());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn preserves_lf_files() {
    let dir = case_dir("lf");
    let path = dir.join("file.txt");
    let mut buffer = open_file(&dir, "file.txt", "a\nb\n");
    assert_eq!(buffer.line_ending(), LineEnding::Lf);
    buffer.save(false).expect("save");
    assert_eq!(fs::read(&path).expect("read"), b"a\nb\n".to_vec());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn counts_crlf_bytes() {
    let buffer = Buffer::from_string("a\r\nb\r\n");
    assert_eq!(buffer.file_bytes(), 6);
}

#[test]
fn normalizes_crlf_from_replacements_and_pastes() {
    let mut buffer = Buffer::from_string("x");
    buffer.replace_all("a\r\nb");
    assert_eq!(buffer.line(0).as_deref(), Some("a"));
    assert_eq!(buffer.line(1).as_deref(), Some("b"));
    buffer.paste_before(0, 0, "c\r\nd");
    assert_eq!(buffer.line(0).as_deref(), Some("c"));
    assert_eq!(buffer.line(1).as_deref(), Some("da"));
    assert!(!buffer.line(1).expect("line").contains('\r'));
}
