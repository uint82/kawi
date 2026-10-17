use crossterm::event::{KeyCode, KeyEvent};

use kawi_editor::{Action, Mode};

use crate::keymap::{
    has_ctrl_or_alt, map_command_mode, map_search_mode, normalize_interrupt, Keymap,
};

pub struct HelixKeymap;

impl HelixKeymap {
    pub fn new() -> Self {
        Self
    }

    fn map_insert(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::ExitInsert,
            KeyCode::Enter => Action::InsertNewline,
            KeyCode::Tab => Action::InsertTab,
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

    fn map_select(&mut self, key: KeyEvent, mode: Mode) -> Action {
        if matches!(key.code, KeyCode::Char('v')) {
            return self.enter_select(key, mode);
        }
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
            KeyCode::Char('i') => self.map_plain_or_noop(key, Action::InsertBeforeSelection),
            KeyCode::Char('a') => self.map_plain_or_noop(key, Action::InsertAfterSelection),
            KeyCode::Char('I') => self.map_plain_or_noop(key, Action::InsertLineStart),
            KeyCode::Char('A') => self.map_plain_or_noop(key, Action::AppendLineEnd),
            KeyCode::Char('o') => self.map_plain_or_noop(key, Action::OpenLineBelow),
            KeyCode::Char('d') => self.map_plain_or_noop(key, Action::DeleteSelection),
            KeyCode::Char('y') => self.map_plain_or_noop(key, Action::YankSelection),
            KeyCode::Char('p') => self.map_plain_or_noop(key, Action::PasteAfter),
            KeyCode::Char('u') => self.map_plain_or_noop(key, Action::Undo),
            KeyCode::Char('U') => self.map_plain_or_noop(key, Action::Redo),
            KeyCode::Char('/') => self.map_plain_or_noop(key, Action::EnterSearch),
            KeyCode::Char('n') => self.map_plain_or_noop(key, Action::NextMatch),
            KeyCode::Char('N') => self.map_plain_or_noop(key, Action::PrevMatch),
            KeyCode::Char(';') => self.map_plain_or_noop(key, Action::CollapseSelection),
            KeyCode::Char(':') => self.map_plain_or_noop(key, Action::EnterCommand),
            _ => Action::Noop,
        }
    }

    fn map_plain_or_noop(&mut self, key: KeyEvent, action: Action) -> Action {
        if has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        action
    }

    fn enter_select(&mut self, key: KeyEvent, mode: Mode) -> Action {
        if mode != Mode::Normal || has_ctrl_or_alt(key) {
            return Action::Noop;
        }
        Action::EnterVisual
    }
}

impl Keymap for HelixKeymap {
    fn map(&mut self, key: KeyEvent, mode: Mode) -> Action {
        let key = normalize_interrupt(key);
        match mode {
            Mode::Insert => self.map_insert(key),
            Mode::Command => map_command_mode(key),
            Mode::Search => map_search_mode(key),
            Mode::Normal | Mode::Visual | Mode::VisualLine => self.map_select(key, mode),
        }
    }

    fn mode_label(&self, mode: Mode) -> &'static str {
        match mode {
            Mode::Visual => "SELECT",
            _ => mode.label(),
        }
    }

    fn operator_exits_visual(&self) -> bool {
        false
    }
}

impl Default for HelixKeymap {
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

    #[test]
    fn select_extends_on_motion() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('l')), Mode::Visual),
            Action::MoveRight
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('w')), Mode::Visual),
            Action::MoveToNextWordStart
        );
    }

    #[test]
    fn insert_tab_maps() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Tab), Mode::Insert),
            Action::InsertTab
        );
    }

    #[test]
    fn insert_keys_collapse() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('i')), Mode::Visual),
            Action::InsertBeforeSelection
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('a')), Mode::Visual),
            Action::InsertAfterSelection
        );
    }

    #[test]
    fn operators_act_on_selection() {
        let mut map = HelixKeymap::new();
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
    fn esc_exits_select_mode() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Visual),
            Action::ExitVisual
        );
        assert_eq!(
            map.map(plain(KeyCode::Esc), Mode::Normal),
            Action::ExitVisual
        );
    }

    #[test]
    fn semicolon_collapses() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char(';')), Mode::Visual),
            Action::CollapseSelection
        );
    }

    #[test]
    fn o_opens_below() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('o')), Mode::Visual),
            Action::OpenLineBelow
        );
    }

    #[test]
    fn line_ends_insert() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('I')), Mode::Visual),
            Action::InsertLineStart
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('A')), Mode::Visual),
            Action::AppendLineEnd
        );
    }

    #[test]
    fn p_pastes_after() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('p')), Mode::Visual),
            Action::PasteAfter
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('p')), Mode::Normal),
            Action::PasteAfter
        );
    }

    #[test]
    fn undo_redo_pair() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('u')), Mode::Visual),
            Action::Undo
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('U')), Mode::Visual),
            Action::Redo
        );
    }

    #[test]
    fn visual_labels_select() {
        let map = HelixKeymap::new();
        assert_eq!(map.mode_label(Mode::Visual), "SELECT");
        assert_eq!(map.mode_label(Mode::Insert), "INSERT");
    }

    #[test]
    fn v_enters_select_from_normal_only() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Normal),
            Action::EnterVisual
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('v')), Mode::Visual),
            Action::Noop
        );
    }

    #[test]
    fn operators_keep_selection() {
        let map = HelixKeymap::new();
        assert!(!map.operator_exits_visual());
    }

    #[test]
    fn search_keys_map_in_select() {
        let mut map = HelixKeymap::new();
        assert_eq!(
            map.map(plain(KeyCode::Char('/')), Mode::Visual),
            Action::EnterSearch
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('n')), Mode::Visual),
            Action::NextMatch
        );
        assert_eq!(
            map.map(plain(KeyCode::Char('N')), Mode::Visual),
            Action::PrevMatch
        );
    }
}
