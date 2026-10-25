use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use kawi_terminal::SpawnOptions;
use kawi_tui::frame::TerminalColor;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use super::frame::line_at;
use super::{spawn_options, TerminalLayout, TerminalMode, TerminalUi};
use crate::app::App;
use crate::keymap::Flavor;

fn options(command: &str) -> SpawnOptions {
    let mut options = spawn_options(std::env::temp_dir(), 10, 40, Some(command.to_string()));
    options.shell = "/bin/sh".to_string();
    options.editor = None;
    options
}

fn wait_for_text(terminal: &mut TerminalUi, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        terminal.drain();
        let text = terminal
            .lines(40, 10)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        if text.contains(needle) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("terminal never printed {needle:?}");
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
#[cfg(unix)]
fn opens_and_reads_output() {
    let (wake, _wake_rx) = channel();
    let mut terminal = TerminalUi::default();
    terminal.open(options("printf hello"), wake).unwrap();
    wait_for_text(&mut terminal, "hello");
    terminal.close();
    assert!(!terminal.is_open());
}

#[test]
#[cfg(unix)]
fn control_backslash_control_n_enters_normal_mode() {
    let (wake, _wake_rx) = channel();
    let mut terminal = TerminalUi::default();
    terminal.open(options("sleep 1"), wake).unwrap();
    assert_eq!(terminal.mode, TerminalMode::Input);
    terminal.handle_key(key(KeyCode::Char('4'), KeyModifiers::CONTROL), 10);
    terminal.handle_key(key(KeyCode::Char('n'), KeyModifiers::CONTROL), 10);
    assert_eq!(terminal.mode, TerminalMode::Normal);
    assert!(terminal.cursor().is_some());
    terminal.handle_key(key(KeyCode::Char('i'), KeyModifiers::NONE), 10);
    assert_eq!(terminal.mode, TerminalMode::Input);
    terminal.close();
}

#[test]
#[cfg(unix)]
fn normal_mode_scrolls_scrollback() {
    let (wake, _wake_rx) = channel();
    let mut terminal = TerminalUi::default();
    terminal
        .open(
            options("printf '1\\n2\\n3\\n4\\n5\\n6\\n7\\n8\\n9\\n10\\n'"),
            wake,
        )
        .unwrap();
    wait_for_text(&mut terminal, "10");
    terminal.handle_key(key(KeyCode::Char('4'), KeyModifiers::CONTROL), 10);
    terminal.handle_key(key(KeyCode::Char('n'), KeyModifiers::CONTROL), 10);
    terminal.handle_key(key(KeyCode::Char('k'), KeyModifiers::NONE), 10);
    let session = terminal.session.as_ref().unwrap();
    assert!(session.screen().scrollback() > 0);
    terminal.close();
}

#[test]
#[cfg(unix)]
fn exit_switches_to_normal_and_reports_status() {
    let (wake, _wake_rx) = channel();
    let mut terminal = TerminalUi::default();
    terminal.open(options("exit 7"), wake).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && terminal.mode != TerminalMode::Normal {
        terminal.drain();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(terminal.mode, TerminalMode::Normal);
    assert_eq!(terminal.status(), "process exited: 7");
    terminal.close();
}

#[test]
fn line_builder_keeps_spans_and_wide_cells() {
    let mut screen = kawi_terminal::TerminalScreen::new(2, 10, 10);
    screen.process(b"ab\x1b[31mcd");
    let line = line_at(&screen, 0, 10);
    assert_eq!(line.spans[0].text, "ab");
    assert_eq!(line.spans[1].text, "cd");
    assert_eq!(line.spans[1].foreground, TerminalColor::Indexed(1));
}

#[test]
fn line_builder_keeps_wide_characters_once() {
    let mut screen = kawi_terminal::TerminalScreen::new(2, 10, 10);
    screen.process("你a".as_bytes());
    let line = line_at(&screen, 0, 10);
    let text = line
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<String>();
    assert_eq!(text.trim_end(), "你a");
}

#[test]
fn panel_height_respects_task_panel() {
    let terminal = TerminalUi {
        open: true,
        ..TerminalUi::default()
    };
    assert_eq!(terminal.panel_height(24, 0), 8);
    assert_eq!(terminal.panel_height(24, 20), 0);
    assert_eq!(terminal.panel_height(4, 0), 0);
}

#[test]
#[cfg(unix)]
fn opens_terminal_pane_in_app() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.open_terminal(
        Some("printf app-terminal-ok; sleep 5".to_string()),
        TerminalLayout::Horizontal,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut text = String::new();
    while Instant::now() < deadline {
        app.sync_terminal();
        text = app
            .terminal
            .lines(100, 8)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        if text.contains("app-terminal-ok") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.terminal_panel_rect().is_some());
    assert!(text.contains("app-terminal-ok"));
    app.close_terminal();
}

#[test]
#[cfg(unix)]
fn app_closes_terminal_after_exit() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.open_terminal(Some("exit 7".to_string()), TerminalLayout::Horizontal)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && app.terminal.is_open() {
        app.sync_terminal();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!app.terminal.is_open());
    assert!(app.terminal_panel_rect().is_none());
    assert_eq!(app.last_message(), Some("Terminal: process exited: 7"));
}

#[test]
#[cfg(unix)]
fn vertical_terminal_uses_the_right_side() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.open_terminal(Some("sleep 5".to_string()), TerminalLayout::Vertical)
        .unwrap();
    let rect = app.terminal_panel_rect().unwrap();
    assert_eq!(rect.y, 0);
    assert_eq!(rect.height, 29);
    assert_eq!(rect.x + rect.width, 100);
    assert!(rect.width < 100);
    let area = app.window_area();
    assert_eq!(area.width + rect.width, 100);
    assert_eq!(area.height, 29);
    app.close_terminal();
}

#[test]
#[cfg(unix)]
fn switching_terminal_layout_keeps_the_session() {
    let mut app = App::new(Buffer::from_string("editor"), Flavor::Vim);
    app.set_screen_size(100, 30);
    app.open_terminal(Some("sleep 5".to_string()), TerminalLayout::Horizontal)
        .unwrap();
    let title = app.terminal.title().to_string();
    let bottom = app.terminal_panel_rect().unwrap();
    app.open_terminal(None, TerminalLayout::Vertical).unwrap();
    let side = app.terminal_panel_rect().unwrap();
    assert_eq!(bottom.y + bottom.height, 29);
    assert_eq!(side.y, 0);
    assert!(side.x > 0);
    assert_eq!(app.terminal.title(), title);
    app.close_terminal();
}
