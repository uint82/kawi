use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::{parse, Mode, RegisterKind};

use super::App;
use crate::app::macro_codec::{decode_keys, encode_key};
use crate::keymap::Keymap;

const MACRO_DEPTH_LIMIT: usize = 100;

struct Recording {
    register: char,
    keys: String,
}

pub(super) struct MacroEngine {
    recording: Option<Recording>,
    last_recorded: Option<char>,
    last_executed: Option<char>,
    depth: usize,
}

impl MacroEngine {
    pub(super) fn new() -> Self {
        Self {
            recording: None,
            last_recorded: None,
            last_executed: None,
            depth: 0,
        }
    }

    pub(super) fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub(super) fn recording_register(&self) -> Option<char> {
        self.recording.as_ref().map(|recording| recording.register)
    }

    pub(super) fn start(&mut self, register: char) {
        self.recording = Some(Recording {
            register,
            keys: String::new(),
        });
    }

    pub(super) fn stop(&mut self) -> Option<(char, String)> {
        let recording = self.recording.take()?;
        self.last_recorded = Some(recording.register);
        Some((recording.register, recording.keys))
    }

    pub(super) fn record_key(&mut self, key: KeyEvent) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        if let Some(encoded) = encode_key(key) {
            recording.keys.push_str(&encoded);
        }
    }

    pub(super) fn last_recorded(&self) -> Option<char> {
        self.last_recorded
    }

    pub(super) fn last_executed(&self) -> Option<char> {
        self.last_executed
    }

    pub(super) fn set_last_executed(&mut self, register: char) {
        self.last_executed = Some(register);
    }

    fn enter_replay(&mut self) -> bool {
        if self.depth >= MACRO_DEPTH_LIMIT {
            return false;
        }
        self.depth += 1;
        true
    }

    fn leave_replay(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}

fn macro_source(text: &str, kind: RegisterKind) -> String {
    if kind != RegisterKind::LineWise {
        return text.to_string();
    }
    let mut source = String::new();
    for line in text.lines() {
        source.push_str(line);
        source.push('\r');
    }
    source
}

impl App {
    pub(super) fn start_macro_recording(&mut self, register: char) {
        self.editor.take_count();
        self.macros.start(register);
    }

    pub(super) fn stop_macro_recording(&mut self) {
        if let Some((register, keys)) = self.macros.stop() {
            self.editor.store_macro_register(register, keys);
        }
    }

    pub(super) fn run_macro(&mut self, register: char) -> Result<bool> {
        let count = self.editor.take_count();
        self.macros.set_last_executed(register);
        if register == '_' {
            return Ok(false);
        }
        let Some((text, kind)) = self.editor.macro_register(register) else {
            return Ok(false);
        };
        let source = macro_source(text, kind);
        self.replay_macro(&source, count)
    }

    pub(super) fn repeat_last_macro(&mut self) -> Result<bool> {
        let Some(register) = self.macros.last_executed() else {
            self.show_message("E748: No previously used register".to_string());
            return Ok(false);
        };
        if register == ':' {
            return self.execute_last_command();
        }
        self.run_macro(register)
    }

    pub(super) fn repeat_recorded_macro(&mut self) -> Result<bool> {
        let Some(register) = self.macros.last_recorded() else {
            return Ok(false);
        };
        self.run_macro(register)
    }

    pub(super) fn execute_last_command(&mut self) -> Result<bool> {
        self.macros.set_last_executed(':');
        let Some(text) = self.last_command.clone() else {
            self.show_message("E30: No previous command line".to_string());
            return Ok(false);
        };
        let count = self.editor.take_count();
        for _ in 0..count {
            let command = parse(&text);
            if self.run_command(command)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn report_macro_register_error(&mut self, register: char) {
        self.show_message(format!("E354: Invalid register name: '{register}'"));
    }

    fn replay_macro(&mut self, text: &str, count: usize) -> Result<bool> {
        // TODO: abort the remaining keys when a replayed key fails, like nvim
        if !self.macros.enter_replay() {
            return Ok(false);
        }
        let keys = decode_keys(text);
        let mut quit = false;
        'replay: for _ in 0..count {
            for key in &keys {
                if self.process_key(*key)? {
                    quit = true;
                    break 'replay;
                }
            }
        }
        self.macros.leave_replay();
        Ok(quit)
    }

    pub(super) fn stops_macro_recording(&self, key: KeyEvent) -> bool {
        if !self.macros.is_recording() || self.pager.is_some() || self.editor.mode() != Mode::Normal
        {
            return false;
        }
        if key.code != KeyCode::Char('q') || key.modifiers != KeyModifiers::NONE {
            return false;
        }
        match self.keymap.pending_keys() {
            None => true,
            Some(pending) => pending.chars().all(|c| c.is_ascii_digit()),
        }
    }

    pub(super) fn consume_macro_stop(&mut self) {
        self.keymap.reset_pending();
        self.editor.take_count();
        self.stop_macro_recording();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with(text: &str) -> App {
        App::new(Buffer::from_string(text), Flavor::Vim)
    }

    fn feed(app: &mut App, keys: &[KeyEvent]) {
        for pressed in keys {
            app.handle_key(*pressed).expect("key");
        }
    }

    fn feed_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_key(key(KeyCode::Char(c))).expect("key");
        }
    }

    fn lines(app: &App) -> Vec<String> {
        app.editor.visible_lines(0, app.editor.line_count())
    }

    fn register(app: &App, name: char) -> Option<String> {
        app.editor
            .macro_register(name)
            .map(|(text, _)| text.to_string())
    }

    fn record_append(app: &mut App, register: char, text: &str) {
        feed(
            app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char(register)),
                key(KeyCode::Char('A')),
            ],
        );
        feed_text(app, text);
        feed(app, &[key(KeyCode::Esc), key(KeyCode::Char('q'))]);
    }

    #[test]
    fn records_and_replays_an_append_macro() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'a', "X");
        assert_eq!(register(&app, 'a'), Some("AX\u{1b}".to_string()));
        assert_eq!(lines(&app), vec!["alphaX"]);
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        assert_eq!(lines(&app), vec!["alphaXX"]);
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        assert_eq!(lines(&app), vec!["alphaXXX"]);
    }

    #[test]
    fn counted_replay_repeats_the_macro() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'a', "X");
        assert_eq!(lines(&app), vec!["alphaX"]);
        feed(
            &mut app,
            &[
                key(KeyCode::Char('2')),
                key(KeyCode::Char('@')),
                key(KeyCode::Char('a')),
            ],
        );
        assert_eq!(lines(&app), vec!["alphaXXX"]);
    }

    #[test]
    fn double_at_repeats_the_last_executed_macro() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'a', "X");
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('@'))],
        );
        assert_eq!(lines(&app), vec!["alphaXXX"]);
    }

    #[test]
    fn shift_q_repeats_the_last_recorded_macro() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'a', "X");
        feed(&mut app, &[key(KeyCode::Char('Q'))]);
        assert_eq!(lines(&app), vec!["alphaXX"]);
    }

    #[test]
    fn uppercase_registers_append_recordings() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'a', "X");
        assert_eq!(lines(&app), vec!["alphaX"]);
        record_append(&mut app, 'A', "Y");
        assert_eq!(lines(&app), vec!["alphaXY"]);
        assert_eq!(register(&app, 'a'), Some("AX\u{1b}AY\u{1b}".to_string()));
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        assert_eq!(lines(&app), vec!["alphaXYXY"]);
    }

    #[test]
    fn empty_macros_and_unset_registers_do_nothing() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char('a')),
                key(KeyCode::Char('q')),
            ],
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('z'))],
        );
        assert_eq!(lines(&app), vec!["alpha"]);
        assert_eq!(app.message, None);
    }

    #[test]
    fn reports_missing_and_invalid_registers() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('@'))],
        );
        assert_eq!(
            app.message.as_deref(),
            Some("E748: No previously used register")
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('%'))],
        );
        assert_eq!(
            app.message.as_deref(),
            Some("E354: Invalid register name: '%'")
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char(':'))],
        );
        assert_eq!(
            app.message.as_deref(),
            Some("E30: No previous command line")
        );
    }

    #[test]
    fn at_colon_reruns_the_last_command() {
        let mut app = app_with("alpha");
        feed_text(&mut app, ":ls");
        feed(&mut app, &[key(KeyCode::Enter)]);
        let listed = app.message.clone();
        assert!(listed.is_some());
        app.message = None;
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char(':'))],
        );
        assert_eq!(app.message, listed);
    }

    #[test]
    fn recording_captures_command_line_keys() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[key(KeyCode::Char('q')), key(KeyCode::Char('f'))],
        );
        feed_text(&mut app, ":set nu");
        feed(&mut app, &[key(KeyCode::Enter), key(KeyCode::Char('q'))]);
        assert_eq!(register(&app, 'f'), Some(":set nu\r".to_string()));
    }

    #[test]
    fn ctrl_c_is_recorded_and_recording_continues() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char('c')),
                key(KeyCode::Char('A')),
            ],
        );
        feed_text(&mut app, "X");
        feed(
            &mut app,
            &[KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)],
        );
        feed_text(&mut app, "Y");
        feed(&mut app, &[key(KeyCode::Esc), key(KeyCode::Char('q'))]);
        assert_eq!(register(&app, 'c'), Some("AX\u{3}Y\u{1b}".to_string()));
    }

    #[test]
    fn count_digit_before_q_is_recorded_and_stops() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char('d')),
                key(KeyCode::Char('A')),
            ],
        );
        feed_text(&mut app, "D");
        feed(&mut app, &[key(KeyCode::Esc)]);
        feed(
            &mut app,
            &[key(KeyCode::Char('2')), key(KeyCode::Char('q'))],
        );
        assert!(!app.macros.is_recording());
        feed(&mut app, &[key(KeyCode::Char('x'))]);
        assert_eq!(register(&app, 'd'), Some("AD\u{1b}2".to_string()));
    }

    #[test]
    fn register_prefix_does_not_stop_recording() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char('b')),
                key(KeyCode::Char('A')),
            ],
        );
        feed_text(&mut app, "B");
        feed(
            &mut app,
            &[
                key(KeyCode::Esc),
                key(KeyCode::Char('"')),
                key(KeyCode::Char('q')),
            ],
        );
        assert!(app.macros.is_recording());
        feed(&mut app, &[key(KeyCode::Char('q'))]);
        assert!(!app.macros.is_recording());
        assert_eq!(register(&app, 'b'), Some("AB\u{1b}\"q".to_string()));
    }

    #[test]
    fn replaying_during_recording_records_the_trigger() {
        let mut app = app_with("alpha");
        record_append(&mut app, 'g', "G");
        feed(
            &mut app,
            &[key(KeyCode::Char('q')), key(KeyCode::Char('h'))],
        );
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('g'))],
        );
        feed(&mut app, &[key(KeyCode::Char('q'))]);
        assert_eq!(register(&app, 'h'), Some("@g".to_string()));
        assert_eq!(lines(&app), vec!["alphaGG"]);
    }

    #[test]
    fn self_referencing_macros_stop_at_the_depth_limit() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[
                key(KeyCode::Char('q')),
                key(KeyCode::Char('a')),
                key(KeyCode::Char('@')),
                key(KeyCode::Char('a')),
                key(KeyCode::Char('q')),
            ],
        );
        assert_eq!(register(&app, 'a'), Some("@a".to_string()));
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('a'))],
        );
        assert_eq!(lines(&app), vec!["alpha"]);
    }

    #[test]
    fn recorded_special_keys_replay() {
        let mut app = app_with("first\nsecond");
        feed(
            &mut app,
            &[key(KeyCode::Char('q')), key(KeyCode::Char('i'))],
        );
        feed(&mut app, &[key(KeyCode::Down)]);
        feed(&mut app, &[key(KeyCode::Esc), key(KeyCode::Char('q'))]);
        assert_eq!(register(&app, 'i'), Some("\u{80}D\u{1b}".to_string()));
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('i'))],
        );
        assert_eq!(app.editor.row(), 1);
    }

    #[test]
    fn yanked_text_plays_back_as_keys() {
        let mut app = app_with("ix\nsecond");
        feed(
            &mut app,
            &[key(KeyCode::Char('y')), key(KeyCode::Char('y'))],
        );
        feed(&mut app, &[key(KeyCode::Char('j'))]);
        feed(
            &mut app,
            &[key(KeyCode::Char('@')), key(KeyCode::Char('"'))],
        );
        assert_eq!(lines(&app), vec!["ix", "x", "second"]);
        assert_eq!(app.editor.mode(), Mode::Insert);
        feed(&mut app, &[key(KeyCode::Esc)]);
    }

    #[test]
    fn showcmd_text_reports_recording() {
        let mut app = app_with("alpha");
        feed(
            &mut app,
            &[key(KeyCode::Char('q')), key(KeyCode::Char('a'))],
        );
        assert_eq!(app.showcmd_text().as_deref(), Some("recording @a"));
        feed(&mut app, &[key(KeyCode::Char('q'))]);
        assert_eq!(app.showcmd_text(), None);
    }
}
