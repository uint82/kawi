use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;
use crate::mode::Mode;

pub trait Keymap {
    fn map(&self, key: KeyEvent, mode: Mode) -> Action;
}

pub struct VimKeymap;

impl Keymap for VimKeymap {
    fn map(&self, key: KeyEvent, mode: Mode) -> Action {
        if is_ctrl_quit(key) {
            return Action::Quit;
        }
        if is_save_key(key) {
            return Action::Save;
        }
        match mode {
            Mode::Insert => map_insert(key),
            Mode::Normal => map_normal(key),
        }
    }
}

fn map_insert(key: KeyEvent) -> Action {
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
        KeyCode::Char(c) => map_insert_char(key, c),
        _ => Action::Noop,
    }
}

fn map_insert_char(key: KeyEvent, c: char) -> Action {
    if has_ctrl_or_alt(key) {
        return Action::Noop;
    }
    Action::InsertChar(c)
}

fn map_normal(key: KeyEvent) -> Action {
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
        KeyCode::Char('h') => Action::MoveLeft,
        KeyCode::Char('j') => Action::MoveDown,
        KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Char('l') => Action::MoveRight,
        KeyCode::Char('i') => map_normal_i(key),
        _ => Action::Noop,
    }
}

fn map_normal_i(key: KeyEvent) -> Action {
    if has_ctrl_or_alt(key) {
        return Action::Noop;
    }
    Action::EnterInsert
}

fn is_ctrl_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('c'))
}

fn is_save_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s'))
}

fn has_ctrl_or_alt(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn normal_h_moves_left() {
        let map = VimKeymap;
        assert_eq!(
            map.map(plain(KeyCode::Char('h')), Mode::Normal),
            Action::MoveLeft
        );
    }

    #[test]
    fn normal_i_enters_insert() {
        let map = VimKeymap;
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Normal),
            Action::EnterInsert
        );
    }

    #[test]
    fn insert_esc_exits() {
        let map = VimKeymap;
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Insert),
            Action::ExitInsert
        );
    }

    #[test]
    fn insert_char_types() {
        let map = VimKeymap;
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Insert),
            Action::InsertChar('q')
        );
    }

    #[test]
    fn normal_q_is_noop() {
        let map = VimKeymap;
        assert_eq!(
            map.map(plain(KeyCode::Char('q')), Mode::Normal),
            Action::Noop
        );
    }

    #[test]
    fn ctrl_q_quits_in_both_modes() {
        let map = VimKeymap;
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Normal),
            Action::Quit
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Insert),
            Action::Quit
        );
    }
}
