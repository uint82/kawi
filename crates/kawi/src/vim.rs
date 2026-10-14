use crossterm::event::{KeyCode, KeyEvent};

use crate::actions::Action;
use crate::keymap::{has_ctrl_or_alt, is_ctrl_quit, is_save_key, Keymap};
use crate::mode::Mode;

pub struct VimKeymap {
    pending_d: bool,
}

impl VimKeymap {
    pub fn new() -> Self {
        Self { pending_d: false }
    }

    fn map_insert(&mut self, key: KeyEvent) -> Action {
        self.clear_pending();
        match key.code {
            KeyCode::Esc => Action::ExitInsert,
            KeyCode::Enter => Action::InsertNewline,
            KeyCode::Backspace => Action::DeleteBackward,
            KeyCode::Delete => Action::DeleteForward,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Char(c) => self.map_insert_char(key, c),
            _ => Action::Noop,
        }
    }

    fn map_insert_char(&mut self, key: KeyEvent, c: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::InsertChar(c)
    }

    fn map_normal(&mut self, key: KeyEvent) -> Action {
        if self.pending_d {
            return self.resolve_d(key);
        }
        self.map_normal_fresh(key)
    }

    fn map_normal_fresh(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Quit,
            KeyCode::Enter => Action::MoveDown,
            KeyCode::Backspace => Action::MoveLeft,
            KeyCode::Delete => Action::DeleteForward,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Char('h') => self.map_plain_or_noop(key, Action::MoveLeft),
            KeyCode::Char('j') => self.map_plain_or_noop(key, Action::MoveDown),
            KeyCode::Char('k') => self.map_plain_or_noop(key, Action::MoveUp),
            KeyCode::Char('l') => self.map_plain_or_noop(key, Action::MoveRight),
            KeyCode::Char('x') => self.map_plain_or_noop(key, Action::DeleteForward),
            KeyCode::Char('0') => self.map_plain_or_noop(key, Action::MoveToLineStart),
            KeyCode::Char('$') => self.map_plain_or_noop(key, Action::MoveToLineEnd),
            KeyCode::Char('i') => self.map_plain_or_noop(key, Action::EnterInsert),
            KeyCode::Char('a') => self.map_plain_or_noop(key, Action::Append),
            KeyCode::Char('o') => self.map_plain_or_noop(key, Action::OpenLineBelow),
            KeyCode::Char('d') => self.arm_d(key),
            KeyCode::Char(':') => self.arm_command(key),
            _ => Action::Noop,
        }
    }

    fn map_plain_or_noop(&mut self, key: KeyEvent, action: Action) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        action
    }

    fn arm_d(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending_d = true;
        Action::Noop
    }

    fn arm_command(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::EnterCommand
    }

    fn resolve_d(&mut self, key: KeyEvent) -> Action {
        self.clear_pending();
        match key.code {
            KeyCode::Char('d') => self.resolve_dd(key),
            _ => Action::Noop,
        }
    }

    fn resolve_dd(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::DeleteLine
    }

    fn map_command(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::CancelCommand,
            KeyCode::Enter => Action::ExecuteCommand,
            KeyCode::Backspace => Action::DeleteCommandChar,
            KeyCode::Char(c) => self.map_command_char(key, c),
            _ => Action::Noop,
        }
    }

    fn map_command_char(&mut self, key: KeyEvent, c: char) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::PushCommandChar(c)
    }

    fn clear_pending(&mut self) {
        self.pending_d = false;
    }
}

impl Keymap for VimKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        if is_ctrl_quit(key) {
            self.clear_pending();
            return Action::Quit;
        }
        if is_save_key(key) {
            self.clear_pending();
            return Action::Save;
        }
        match mode {
            Mode::Insert => self.map_insert(key),
            Mode::Normal => self.map_normal(key),
            Mode::Command => self.map_command(key),
        }
    }
}

impl Default for VimKeymap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn normal_h_moves_left() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('h')), Mode::Normal),
            Action::MoveLeft
        );
    }

    #[test]
    fn normal_i_enters_insert() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::EnterInsert
        );
    }

    #[test]
    fn insert_esc_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Insert),
            Action::ExitInsert
        );
    }

    #[test]
    fn insert_char_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Insert),
            Action::InsertChar('q')
        );
    }

    #[test]
    fn normal_q_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Normal),
            Action::Noop
        );
    }

    #[test]
    fn ctrl_q_quits_in_both_modes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Normal),
            Action::Quit
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Insert),
            Action::Quit
        );
    }

    #[test]
    fn normal_x_deletes_forward() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('x')), Mode::Normal),
            Action::DeleteForward
        );
    }

    #[test]
    fn normal_dollar_moves_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('$')), Mode::Normal),
            Action::MoveToLineEnd
        );
    }

    #[test]
    fn dd_deletes_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::DeleteLine
        );
    }

    #[test]
    fn d_then_other_clears() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Normal),
            Action::MoveDown
        );
    }

    #[test]
    fn colon_enters_command() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char(':')), Mode::Normal),
            Action::EnterCommand
        );
    }

    #[test]
    fn command_chars_push() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Command),
            Action::PushCommandChar('w')
        );
    }

    #[test]
    fn command_enter_executes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Enter), Mode::Command),
            Action::ExecuteCommand
        );
    }

    #[test]
    fn command_esc_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Command),
            Action::CancelCommand
        );
    }

    #[test]
    fn normal_o_opens_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('o')), Mode::Normal),
            Action::OpenLineBelow
        );
    }

    #[test]
    fn normal_a_appends() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Normal),
            Action::Append
        );
    }

    #[test]
    fn insert_a_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Insert),
            Action::InsertChar('a')
        );
    }
}
