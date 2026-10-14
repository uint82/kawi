use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::keymap::{has_ctrl_or_alt, normalize_interrupt, Keymap};
use kawi_editor::Action;
use kawi_editor::Mode;

fn is_redo_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('r'))
}

pub struct VimKeymap {
    pending_d: bool,
    pending_c: bool,
    pending_y: bool,
}

impl VimKeymap {
    pub fn new() -> Self {
        Self {
            pending_d: false,
            pending_c: false,
            pending_y: false,
        }
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
        if self.pending_c {
            return self.resolve_c(key);
        }
        if self.pending_d {
            return self.resolve_d(key);
        }
        if self.pending_y {
            return self.resolve_y(key);
        }
        self.map_normal_fresh(key)
    }

    fn map_normal_fresh(&mut self, key: KeyEvent) -> Action {
        if is_redo_key(key) {
            return Action::Redo;
        }
        match key.code {
            KeyCode::Esc => Action::Noop,
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
            KeyCode::Char('u') => self.map_plain_or_noop(key, Action::Undo),
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::MoveToNextWordStart),
            KeyCode::Char('b') => self.map_plain_or_noop(key, Action::MoveToPrevWordStart),
            KeyCode::Char('e') => self.map_plain_or_noop(key, Action::MoveToNextWordEnd),
            KeyCode::Char('o') => self.map_plain_or_noop(key, Action::OpenLineBelow),
            KeyCode::Char('p') => self.map_plain_or_noop(key, Action::PasteAfter),
            KeyCode::Char('D') => self.map_plain_or_noop(key, Action::DeleteToEnd),
            KeyCode::Char('C') => self.map_plain_or_noop(key, Action::ChangeToEnd),
            KeyCode::Char('Y') => self.map_plain_or_noop(key, Action::YankLine),
            KeyCode::Char('d') => self.arm_d(key),
            KeyCode::Char('c') => self.arm_c(key),
            KeyCode::Char('y') => self.arm_y(key),
            KeyCode::Char(':') => self.arm_command(key),
            KeyCode::Char('v') => self.map_plain_or_noop(key, Action::EnterVisual),
            _ => Action::Noop,
        }
    }

    fn map_plain_or_noop(&mut self, key: KeyEvent, action: Action) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        action
    }

    fn clear_pending(&mut self) {
        self.pending_d = false;
        self.pending_c = false;
        self.pending_y = false;
    }

    fn arm_d(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending_d = true;
        Action::Noop
    }

    fn arm_c(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending_c = true;
        Action::Noop
    }

    fn arm_y(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        self.pending_y = true;
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
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::DeleteWord),
            _ => Action::Noop,
        }
    }

    fn resolve_dd(&mut self, key: KeyEvent) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::DeleteLine
    }

    fn resolve_c(&mut self, key: KeyEvent) -> Action {
        self.clear_pending();
        match key.code {
            KeyCode::Char('c') => self.map_plain_or_noop(key, Action::ChangeLine),
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::ChangeWord),
            _ => Action::Noop,
        }
    }

    fn resolve_y(&mut self, key: KeyEvent) -> Action {
        self.clear_pending();
        match key.code {
            KeyCode::Char('y') => self.map_plain_or_noop(key, Action::YankLine),
            _ => Action::Noop,
        }
    }

    fn map_command(&mut self, key: KeyEvent) -> Action {
        crate::keymap::map_command_mode(key)
    }

    fn map_visual(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::ExitVisual,
            KeyCode::Up => Action::MoveUp,
            KeyCode::Down => Action::MoveDown,
            KeyCode::Left => Action::MoveLeft,
            KeyCode::Right => Action::MoveRight,
            KeyCode::Home => Action::MoveToLineStart,
            KeyCode::End => Action::MoveToLineEnd,
            KeyCode::Char('h') => self.map_plain_or_noop(key, Action::MoveLeft),
            KeyCode::Char('j') => self.map_plain_or_noop(key, Action::MoveDown),
            KeyCode::Char('k') => self.map_plain_or_noop(key, Action::MoveUp),
            KeyCode::Char('l') => self.map_plain_or_noop(key, Action::MoveRight),
            KeyCode::Char('w') => self.map_plain_or_noop(key, Action::MoveToNextWordStart),
            KeyCode::Char('b') => self.map_plain_or_noop(key, Action::MoveToPrevWordStart),
            KeyCode::Char('e') => self.map_plain_or_noop(key, Action::MoveToNextWordEnd),
            KeyCode::Char('d') => self.map_plain_or_noop(key, Action::DeleteSelection),
            KeyCode::Char('y') => self.map_plain_or_noop(key, Action::YankSelection),
            KeyCode::Char('v') => self.map_plain_or_noop(key, Action::ExitVisual),
            _ => Action::Noop,
        }
    }
}

impl Keymap for VimKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        let key = normalize_interrupt(key);
        match mode {
            Mode::Insert => self.map_insert(key),
            Mode::Normal => self.map_normal(key),
            Mode::Command => self.map_command(key),
            Mode::Visual => self.map_visual(key),
        }
    }

    fn operator_exits_visual(&self) -> bool {
        true
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
    fn normal_esc_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(map.map(plain(KeyCode::Esc), Mode::Normal), Action::Noop);
    }

    #[test]
    fn ctrl_s_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('s')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('s')), Mode::Insert),
            Action::Noop
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
    fn ctrl_q_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('q')), Mode::Insert),
            Action::Noop
        );
    }

    #[test]
    fn ctrl_c_matches_esc_everywhere() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Insert),
            Action::ExitInsert
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Visual),
            Action::ExitVisual
        );
        assert_eq!(
            map.map(ctrl(KeyCode::Char('c')), Mode::Command),
            Action::CancelCommand
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
    fn normal_v_enters_visual() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::EnterVisual
        );
    }

    #[test]
    fn visual_esc_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Visual),
            Action::ExitVisual
        );
    }

    #[test]
    fn visual_motion_moves() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('l')), Mode::Visual),
            Action::MoveRight
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('j')), Mode::Visual),
            Action::MoveDown
        );
    }

    #[test]
    fn visual_v_exits() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Visual),
            Action::ExitVisual
        );
    }

    #[test]
    fn normal_u_undoes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Normal),
            Action::Undo
        );
    }

    #[test]
    fn normal_ctrl_r_redoes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('r')), Mode::Normal),
            Action::Redo
        );
    }

    #[test]
    fn insert_u_types() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Insert),
            Action::InsertChar('u')
        );
    }

    #[test]
    fn insert_ctrl_r_is_noop() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(ctrl(KeyCode::Char('r')), Mode::Insert),
            Action::Noop
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

    #[test]
    fn normal_word_motions_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::MoveToNextWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('b')), Mode::Normal),
            Action::MoveToPrevWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Normal),
            Action::MoveToNextWordEnd
        );
    }

    #[test]
    fn visual_word_motions_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Visual),
            Action::MoveToNextWordStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('e')), Mode::Visual),
            Action::MoveToNextWordEnd
        );
    }

    #[test]
    fn visual_operators_map() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Visual),
            Action::DeleteSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Visual),
            Action::YankSelection
        );
    }

    #[test]
    fn normal_p_pastes() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('p')), Mode::Normal),
            Action::PasteAfter
        );
    }

    #[test]
    fn pending_d_then_v_cancels() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::EnterVisual
        );
    }

    #[test]
    fn double_c_changes_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::ChangeLine
        );
    }

    #[test]
    fn c_w_changes_word() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('c')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::ChangeWord
        );
    }

    #[test]
    fn d_w_deletes_word() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('d')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Normal),
            Action::DeleteWord
        );
    }

    #[test]
    fn double_y_yanks_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Normal),
            Action::Noop
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('y')), Mode::Normal),
            Action::YankLine
        );
    }

    #[test]
    fn capital_y_yanks_line() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('Y')), Mode::Normal),
            Action::YankLine
        );
    }

    #[test]
    fn capital_d_deletes_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('D')), Mode::Normal),
            Action::DeleteToEnd
        );
    }

    #[test]
    fn capital_c_changes_to_end() {
        let mut map = VimKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('C')), Mode::Normal),
            Action::ChangeToEnd
        );
    }
}
