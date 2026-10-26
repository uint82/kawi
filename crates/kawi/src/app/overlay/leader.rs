use std::borrow::Cow;

use kawi_tui::frame::Rect;
use kawi_tui::overlay::PANEL_BORDER;
use kawi_tui::text::display_width;

use super::super::App;
use super::{ListData, OverlayData, MAX_LIST_WIDTH};
use crate::keymap::Keymap;
use crate::leader::{self, LeaderPath};

const KEY_LABEL_SEPARATOR: &str = "  ";
const RIGHT_MARGIN: usize = 1;
const BOTTOM_MARGIN: usize = 1;

impl App {
    pub(super) fn leader_overlay(&self) -> Option<OverlayData<'_>> {
        let path = self.keymap.leader_path()?;
        let items = leader_lines(path, self.editor.mode().is_visual());
        if items.is_empty() {
            return None;
        }
        let rect = leader_rect(self.window_area(), &items);
        Some(OverlayData {
            list: Some(ListData {
                rect,
                items: Cow::Owned(items),
                selected: None,
            }),
            panel: None,
        })
    }
}

fn leader_lines(path: LeaderPath, visual: bool) -> Vec<String> {
    match path {
        LeaderPath::Root => leader::groups(visual)
            .map(|group| leader_row(group.key, group.label))
            .collect(),
        LeaderPath::Group(group) => group
            .commands
            .iter()
            .map(|command| leader_row(command.key, command.label))
            .collect(),
    }
}

fn leader_row(key: char, label: &str) -> String {
    format!("{key}{KEY_LABEL_SEPARATOR}{label}")
}

fn leader_rect(area: Rect, items: &[String]) -> Rect {
    let content_width = items
        .iter()
        .map(|item| display_width(item))
        .max()
        .unwrap_or(0);
    let width = content_width
        .saturating_add(PANEL_BORDER)
        .min(MAX_LIST_WIDTH)
        .min(area.width)
        .max(1);
    let height = items
        .len()
        .saturating_add(PANEL_BORDER)
        .min(area.height)
        .max(1);
    let x = area
        .right()
        .saturating_sub(width)
        .saturating_sub(RIGHT_MARGIN)
        .max(area.x);
    let y = area
        .bottom()
        .saturating_sub(height)
        .saturating_sub(BOTTOM_MARGIN)
        .max(area.y);
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;

    use crate::keymap::Flavor;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app() -> App {
        let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
        app.set_screen_size(80, 24);
        app
    }

    fn items(app: &App) -> Vec<String> {
        let data = app.overlay_data().expect("overlay");
        let list = data.list.as_ref().expect("list");
        list.items.to_vec()
    }

    fn root_items() -> [&'static str; 5] {
        ["c  code", "f  find", "r  rename", "t  tasks", "v  git"]
    }

    #[test]
    fn root_menu_lists_every_group() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        assert_eq!(items(&app), root_items());
    }

    #[test]
    fn group_menu_replaces_the_root_menu() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        app.handle_key(key(KeyCode::Char('f'))).expect("group");
        assert_eq!(
            items(&app),
            [
                "f  find project file",
                "k  find buffer",
                "r  find and replace",
                "s  find in project",
                "w  find current word",
            ]
        );
    }

    #[test]
    fn visual_menu_only_lists_supported_groups() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        assert_eq!(items(&app), ["c  code", "f  find"]);
    }

    #[test]
    fn backslash_leader_uses_the_same_menu() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char('\\'))).expect("leader");
        assert_eq!(items(&app), root_items());
        app.handle_key(key(KeyCode::Char('f'))).expect("group");
        assert_eq!(
            items(&app).first().map(String::as_str),
            Some("f  find project file")
        );
    }

    #[test]
    fn completing_a_command_dismisses_the_menu() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        app.handle_key(key(KeyCode::Char('t'))).expect("group");
        app.handle_key(key(KeyCode::Char('c'))).expect("command");
        assert!(app.overlay_data().is_none());
    }

    #[test]
    fn canceling_and_unknown_keys_dismiss_the_menu() {
        let mut app = app();
        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert!(app.overlay_data().is_none());

        app.handle_key(key(KeyCode::Char(' '))).expect("leader");
        app.handle_key(key(KeyCode::Char('z'))).expect("unknown");
        assert!(app.overlay_data().is_none());
    }

    #[test]
    fn menu_is_right_aligned_above_the_bottom_edge() {
        let items = vec!["f  find".to_string(), "v  git".to_string()];
        let rect = leader_rect(Rect::new(0, 0, 80, 24), &items);
        assert_eq!(rect.width, 9);
        assert_eq!(rect.height, 4);
        assert_eq!(rect.x, 70);
        assert_eq!(rect.y, 19);
    }
}
