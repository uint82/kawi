use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy)]
pub(super) struct MenuSelection {
    index: usize,
    len: usize,
}

impl MenuSelection {
    pub(super) fn new(len: usize, preferred: Option<usize>) -> Self {
        Self {
            index: preferred.unwrap_or(0).min(len.saturating_sub(1)),
            len,
        }
    }

    pub(super) fn selected(&self) -> Option<usize> {
        (self.len > 0).then_some(self.index)
    }

    pub(super) fn move_by(&mut self, delta: isize) {
        if self.len == 0 {
            return;
        }
        let len = self.len as isize;
        self.index = (self.index as isize + delta).rem_euclid(len) as usize;
    }
}

pub(super) enum MenuKey {
    Cancel,
    Move(isize),
    Accept,
    Other,
}

pub(super) fn menu_key(key: KeyEvent) -> MenuKey {
    if key.code == KeyCode::Esc
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
    {
        return MenuKey::Cancel;
    }
    match key.code {
        KeyCode::Up => MenuKey::Move(-1),
        KeyCode::Down => MenuKey::Move(1),
        KeyCode::PageUp => MenuKey::Move(-10),
        KeyCode::PageDown => MenuKey::Move(10),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => MenuKey::Move(-1),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => MenuKey::Move(1),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => MenuKey::Move(-1),
        KeyCode::BackTab => MenuKey::Move(-1),
        KeyCode::Tab => MenuKey::Move(1),
        KeyCode::Enter => MenuKey::Accept,
        _ => MenuKey::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_and_clamps() {
        let mut selection = MenuSelection::new(3, Some(9));
        assert_eq!(selection.selected(), Some(2));
        selection.move_by(1);
        assert_eq!(selection.selected(), Some(0));
        selection.move_by(-1);
        assert_eq!(selection.selected(), Some(2));
        assert_eq!(MenuSelection::new(0, None).selected(), None);
    }

    #[test]
    fn navigation_keys_map_to_moves() {
        for (code, delta) in [
            (KeyCode::Up, -1),
            (KeyCode::Down, 1),
            (KeyCode::PageUp, -10),
            (KeyCode::PageDown, 10),
        ] {
            match menu_key(KeyEvent::new(code, KeyModifiers::NONE)) {
                MenuKey::Move(actual) => assert_eq!(actual, delta),
                _ => panic!("expected move"),
            }
        }
    }

    #[test]
    fn tab_and_shift_tab_move_the_selection() {
        assert!(matches!(
            menu_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            MenuKey::Move(1)
        ));
        assert!(matches!(
            menu_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)),
            MenuKey::Move(-1)
        ));
        assert!(matches!(
            menu_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)),
            MenuKey::Move(-1)
        ));
    }
}
