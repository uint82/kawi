use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use kawi_editor::clipboard::{Clipboard, ClipboardKind};

const HELPER_TIMEOUT: Duration = Duration::from_secs(2);

pub struct SystemClipboard;

impl SystemClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Clipboard for SystemClipboard {
    fn read(&mut self, kind: ClipboardKind) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            let _ = kind;
            read_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            read_wayland(kind).or_else(|| read_x11(kind))
        }
    }

    fn write(&mut self, kind: ClipboardKind, text: &str) {
        #[cfg(target_os = "macos")]
        {
            if write_macos(text) {
                return;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if write_wayland(kind, text) || write_x11(kind, text) {
                return;
            }
        }
        if kind == ClipboardKind::Clipboard {
            write_osc52(text);
        }
    }
}

impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "macos"))]
fn primary_flag(kind: ClipboardKind) -> Option<&'static str> {
    match kind {
        ClipboardKind::Primary => Some("--primary"),
        ClipboardKind::Clipboard => None,
    }
}

#[cfg(not(target_os = "macos"))]
fn read_wayland(kind: ClipboardKind) -> Option<String> {
    let args: Vec<&str> = match primary_flag(kind) {
        Some(flag) => vec![flag, "--no-newline"],
        None => vec!["--no-newline"],
    };
    run_output("wl-paste", &args)
}

#[cfg(not(target_os = "macos"))]
fn read_x11(kind: ClipboardKind) -> Option<String> {
    let selection = match kind {
        ClipboardKind::Primary => "primary",
        ClipboardKind::Clipboard => "clipboard",
    };
    run_output("xclip", &["-selection", selection, "-o"])
        .or_else(|| run_output("xsel", &["--output", "--clipboard"]))
}

#[cfg(target_os = "macos")]
fn read_macos() -> Option<String> {
    run_output("pbpaste", &[])
}

#[cfg(not(target_os = "macos"))]
fn write_wayland(kind: ClipboardKind, text: &str) -> bool {
    let mut args: Vec<&str> = Vec::new();
    if let Some(flag) = primary_flag(kind) {
        args.push(flag);
    }
    run_input("wl-copy", &args, text)
}

#[cfg(not(target_os = "macos"))]
fn write_x11(kind: ClipboardKind, text: &str) -> bool {
    let selection = match kind {
        ClipboardKind::Primary => "primary",
        ClipboardKind::Clipboard => "clipboard",
    };
    run_input("xclip", &["-selection", selection], text)
        || run_input("xsel", &["--input", "--clipboard"], text)
}

#[cfg(target_os = "macos")]
fn write_macos(text: &str) -> bool {
    run_input("pbcopy", &[], text)
}

fn run_output(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        stdout.read_to_end(&mut text).ok().map(|_| text)
    });
    if wait_with_timeout(&mut child) != Some(true) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = reader.join();
        return None;
    }
    reader
        .join()
        .ok()
        .flatten()
        .and_then(|text| String::from_utf8(text).ok())
}

fn wait_with_timeout(child: &mut Child) -> Option<bool> {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn run_input(program: &str, args: &[&str], text: &str) -> bool {
    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let written = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all(text.as_bytes()).is_ok(),
        None => false,
    };
    if !written {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }
    wait_with_timeout(&mut child) == Some(true)
}

fn write_osc52(text: &str) {
    let encoded = base64(text.as_bytes());
    let sequence = format!("\x1b]52;c;{}\x07", encoded);
    let mut out = std::io::stdout();
    let _ = out.write_all(sequence.as_bytes());
    let _ = out.flush();
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(triple >> 18) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encodes_text() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        assert_eq!(base64(b"hello world"), "aGVsbG8gd29ybGQ=");
    }
}
