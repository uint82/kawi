use crate::parser::OutputStream;
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::SyncSender;
use std::thread::{self, JoinHandle};

pub(super) const MAX_OUTPUT_LINE_CHARS: usize = 4_096;

pub(super) struct RawLine {
    pub(super) stream: OutputStream,
    pub(super) line: String,
}

pub(super) fn sanitize_line(text: &str) -> String {
    let mut result = String::new();
    let mut characters = text.chars().peekable();
    let mut length = 0;
    while let Some(character) = characters.next() {
        if length >= MAX_OUTPUT_LINE_CHARS {
            break;
        }
        match character {
            '\u{1b}' => skip_escape(&mut characters),
            '\t' => {
                let spaces = (MAX_OUTPUT_LINE_CHARS - length).min(4);
                result.push_str(&" ".repeat(spaces));
                length = length.saturating_add(spaces);
            }
            '\r' => {}
            control if control.is_control() => {}
            text => {
                result.push(text);
                length = length.saturating_add(1);
            }
        }
    }
    result
}

fn skip_escape(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match characters.next() {
        Some('[') => {
            for character in characters.by_ref() {
                if ('@'..='~').contains(&character) {
                    break;
                }
            }
        }
        Some(']') => {
            while let Some(character) = characters.next() {
                if character == '\u{7}' {
                    break;
                }
                if character == '\u{1b}' && characters.peek() == Some(&'\\') {
                    characters.next();
                    break;
                }
            }
        }
        _ => {}
    }
}

pub(super) fn spawn_reader<R: Read + Send + 'static>(
    name: &str,
    reader: R,
    stream: OutputStream,
    sender: SyncSender<RawLine>,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {
                        if bytes.last() == Some(&b'\n') {
                            bytes.pop();
                        }
                        if bytes.last() == Some(&b'\r') {
                            bytes.pop();
                        }
                        let line = sanitize_line(&String::from_utf8_lossy(&bytes));
                        if sender.send(RawLine { stream, line }).is_err() {
                            return;
                        }
                    }
                }
            }
        })
}
