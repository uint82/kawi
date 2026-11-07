use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_editor::{BracketMatch, ViewState, WindowOptions};
use kawi_tui::frame::Rect;

mod app;
mod layout;
mod metadata;
mod snapshot;

#[cfg(test)]
use layout::pane_rect;
pub use layout::WindowLayout;
use layout::{adjust_active, equalize_node, first_leaf, remove_leaf, split_leaf, Node};
pub(crate) use layout::{direction_score, score_precedes};

pub type WindowId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAxis {
    Width,
    Height,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowView {
    pub buffer: usize,
    pub view: ViewState,
    pub options: WindowOptions,
    pub(crate) bracket_match: Option<BracketMatchCache>,
}

impl WindowView {
    pub fn new(buffer: usize, view: ViewState) -> Self {
        Self::with_options(buffer, view, WindowOptions::default())
    }

    pub fn with_options(buffer: usize, view: ViewState, options: WindowOptions) -> Self {
        Self {
            buffer,
            view,
            options,
            bracket_match: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BracketMatchKey {
    pub buffer: usize,
    pub revision: u64,
    pub language_generation: u64,
    pub row: usize,
    pub col: usize,
    pub insert: bool,
    pub top: usize,
    pub leftcol: usize,
    pub width: usize,
    pub height: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BracketMatchCache {
    pub key: BracketMatchKey,
    pub matched: Option<BracketMatch>,
}

pub struct Windows {
    root: Option<Node>,
    windows: BTreeMap<WindowId, WindowView>,
    active: WindowId,
    next_id: WindowId,
}

impl Windows {
    pub fn new(buffer: usize, view: ViewState) -> Self {
        let id = 1;
        let mut windows = BTreeMap::new();
        windows.insert(id, WindowView::new(buffer, view));
        Self {
            root: Some(Node::Leaf(id)),
            windows,
            active: id,
            next_id: id.saturating_add(1),
        }
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn active(&self) -> WindowId {
        self.active
    }

    pub fn active_view(&self) -> &WindowView {
        self.windows
            .get(&self.active)
            .expect("the active window exists")
    }

    pub fn active_view_mut(&mut self) -> &mut WindowView {
        self.windows
            .get_mut(&self.active)
            .expect("the active window exists")
    }

    pub fn view(&self, id: WindowId) -> Option<&WindowView> {
        self.windows.get(&id)
    }

    pub fn set_active(&mut self, id: WindowId) -> bool {
        if !self.windows.contains_key(&id) {
            return false;
        }
        self.active = id;
        true
    }

    pub fn split_active(&mut self, direction: SplitDirection, view: WindowView) -> WindowId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.windows.insert(id, view);
        let root = self.root.as_mut().expect("the window tree exists");
        if !split_leaf(root, self.active, id, direction) {
            self.windows.remove(&id);
            return self.active;
        }
        self.active = id;
        id
    }

    pub fn remove_active(&mut self) -> Option<WindowId> {
        if self.windows.len() <= 1 {
            return None;
        }
        let target = self.active;
        let root = self.root.take()?;
        let (root, focus) = remove_leaf(root, target);
        self.root = root;
        self.windows.remove(&target);
        let focus = focus
            .or_else(|| self.root.as_ref().map(first_leaf))
            .expect("a remaining window has a leaf");
        self.active = focus;
        Some(focus)
    }

    pub fn only_active(&mut self) -> usize {
        let active = self.active;
        let removed = self.windows.len().saturating_sub(1);
        self.windows.retain(|id, _| *id == active);
        self.root = Some(Node::Leaf(active));
        removed
    }

    pub fn layout(&self, area: Rect) -> WindowLayout {
        WindowLayout::compute(self.root.as_ref(), area)
    }

    #[cfg(test)]
    pub fn focus_direction(&self, direction: FocusDirection, area: Rect) -> Option<WindowId> {
        let layout = self.layout(area);
        let current = pane_rect(&layout.panes, self.active)?;
        let mut best: Option<(WindowId, (usize, usize, usize))> = None;
        for (id, rect) in &layout.panes {
            if *id == self.active {
                continue;
            }
            let Some(score) = direction_score(direction, current, *rect) else {
                continue;
            };
            let replace = best.is_none_or(|(_, best_score)| score_precedes(score, best_score));
            if replace {
                best = Some((*id, score));
            }
        }
        best.map(|(id, _)| id)
    }

    pub fn focus_next(&self, area: Rect, reverse: bool) -> Option<WindowId> {
        let layout = self.layout(area);
        let ids: Vec<WindowId> = layout.panes.iter().map(|(id, _)| *id).collect();
        if ids.len() < 2 {
            return None;
        }
        let index = ids.iter().position(|id| *id == self.active)?;
        let next = if reverse {
            index.checked_sub(1).unwrap_or(ids.len() - 1)
        } else {
            (index + 1) % ids.len()
        };
        Some(ids[next])
    }

    pub fn resize_active(&mut self, axis: ResizeAxis, grow: bool) -> bool {
        let root = self.root.as_mut().expect("the window tree exists");
        adjust_active(root, self.active, axis, grow)
    }

    pub fn equalize(&mut self) -> bool {
        let root = self.root.as_mut().expect("the window tree exists");
        equalize_node(root)
    }

    pub(crate) fn set_all_options(&mut self, options: WindowOptions) {
        for window in self.windows.values_mut() {
            window.options = options;
        }
    }

    pub(crate) fn clamp_views<F>(&mut self, mut clamp: F)
    where
        F: FnMut(usize, ViewState) -> ViewState,
    {
        for window in self.windows.values_mut() {
            window.view = clamp(window.buffer, window.view);
        }
    }

    pub fn count_buffer(&self, buffer: usize) -> usize {
        self.windows
            .values()
            .filter(|window| window.buffer == buffer)
            .count()
    }

    pub(crate) fn view_for_buffer(&self, buffer: usize) -> Option<ViewState> {
        if self.active_view().buffer == buffer {
            return Some(self.active_view().view);
        }
        self.windows
            .values()
            .find(|window| window.buffer == buffer)
            .map(|window| window.view)
    }

    pub(crate) fn set_view_for_buffer(&mut self, buffer: usize, view: ViewState) -> usize {
        let mut updated = 0;
        for window in self.windows.values_mut() {
            if window.buffer != buffer {
                continue;
            }
            window.view = view;
            updated += 1;
        }
        updated
    }

    pub fn replace_unlisted_buffers(&mut self, listed: &[usize], fallback: usize) -> usize {
        let mut replaced = 0;
        for window in self.windows.values_mut() {
            if listed.contains(&window.buffer) {
                continue;
            }
            window.buffer = fallback;
            window.view = ViewState::default();
            replaced += 1;
        }
        replaced
    }
}

pub(super) fn is_window_prefix_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('w' | 'W'))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use kawi_buffer::Buffer;
    use kawi_editor::{Action, FoldMethod};

    use super::*;
    use crate::app::App;
    use crate::keymap::Flavor;
    use std::time::Duration;

    const VIEW: ViewState = ViewState {
        row: 0,
        col: 0,
        desired: 0,
        top: 0,
        leftcol: 0,
    };

    fn windows(count: usize) -> Windows {
        let mut windows = Windows::new(1, VIEW);
        for index in 1..count {
            windows.split_active(SplitDirection::Horizontal, WindowView::new(index + 1, VIEW));
        }
        windows
    }

    fn area() -> Rect {
        Rect::new(0, 0, 100, 40)
    }

    fn app(text: &str) -> App {
        App::new(Buffer::from_string(text), Flavor::Vim)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_w() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)
    }

    fn feed(app: &mut App, keys: &[KeyEvent]) {
        for pressed in keys {
            app.handle_key(*pressed).expect("key");
        }
    }

    fn command(app: &mut App, text: &str) {
        app.handle_key(key(KeyCode::Char(':')))
            .expect("command mode");
        for character in text.chars() {
            app.handle_key(key(KeyCode::Char(character)))
                .expect("command key");
        }
        app.handle_key(key(KeyCode::Enter)).expect("command");
    }

    fn press(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn drag(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn syntax_highlighting_survives_splits_and_focus_changes() {
        let mut buffer = Buffer::from_string("fn main() {}\n");
        buffer.path = Some("main.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.refresh_syntax();
        wait_for_syntax(&mut app);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('h'))]);
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        assert_eq!(panes.len(), 2);
        assert!(panes.iter().all(|pane| !pane.syntax_spans.is_empty()));
    }

    #[test]
    fn folded_pane_reports_visible_rows() {
        let buffer = Buffer::from_string(
            "def main():\n    a = 1\n    if True:\n        b = 2\n    c = 3\nd = 4\n",
        );
        let mut app = App::new(buffer, Flavor::Vim);
        app.editor.set_shiftwidth(4);
        app.editor.set_fold_method(FoldMethod::Indent);
        app.editor.close_all_folds();
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        let pane = panes.iter().find(|pane| pane.active).expect("active pane");
        assert_eq!(pane.rows, vec![0, 1, 5]);
        assert_eq!(pane.lines.len(), pane.rows.len());
        assert_eq!(pane.cursor_screen_row, Some(0));
        assert!(pane.gutter[1].starts_with('+'));
    }

    #[test]
    fn inactive_windows_keep_syntax_for_other_buffers() {
        let other = std::env::temp_dir().join(format!(
            "kawi_window_syntax_{}_{}.rs",
            std::process::id(),
            "other"
        ));
        std::fs::write(&other, "fn other() {}\n").unwrap();
        let mut buffer = Buffer::from_string("fn main() {}\n");
        buffer.path = Some("main.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.refresh_syntax();
        wait_for_syntax(&mut app);
        command(&mut app, &format!("vsplit {}", other.to_string_lossy()));
        app.refresh_syntax();
        wait_for_syntax(&mut app);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('h'))]);
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        assert_eq!(panes.len(), 2);
        assert!(panes.iter().all(|pane| !pane.syntax_spans.is_empty()));
        std::fs::remove_file(other).unwrap();
    }

    fn wait_for_syntax(app: &mut App) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            app.apply_syntax_results();
            if !app.syntax_spans.is_empty() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("syntax result timed out");
    }

    #[test]
    fn new_window_opens_an_empty_buffer() {
        let mut app = app("alpha\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('n'))]);
        assert_eq!(app.windows.len(), 2);
        assert_ne!(app.editor.buffer_number(), 1);
        assert_eq!(app.windows.count_buffer(1), 1);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('c'))]);
        assert_eq!(app.windows.len(), 1);
        assert_eq!(app.editor.buffer_number(), 1);
    }

    #[test]
    fn ctrl_w_q_quits_from_the_last_window() {
        let path = std::env::temp_dir().join(format!(
            "kawi_window_quit_{}_{}.txt",
            std::process::id(),
            "clean"
        ));
        std::fs::write(&path, "alpha\n").unwrap();
        let buffer = Buffer::from_file(path.to_str().unwrap()).unwrap();
        let mut app = App::new(buffer, Flavor::Vim);
        feed(&mut app, &[ctrl_w()]);
        let quit = app.handle_key(key(KeyCode::Char('q'))).expect("quit");
        assert!(quit);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ctrl_w_s_splits_and_ctrl_w_c_closes() {
        let mut app = app("alpha\nbeta\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('s'))]);
        assert_eq!(app.windows.len(), 2);
        assert_eq!(app.windows.count_buffer(1), 2);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('c'))]);
        assert_eq!(app.windows.len(), 1);
        assert_eq!(app.windows.count_buffer(1), 1);
    }

    #[test]
    fn vertical_split_focuses_left_and_restores_each_cursor() {
        let mut app = app("alpha\nbeta\ngamma\n");
        app.editor.apply(Action::JumpTo(1, 1));
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        feed(&mut app, &[key(KeyCode::Char('j'))]);
        assert_eq!(app.cursor_position(), (2, 1));
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('h'))]);
        assert_eq!(app.cursor_position(), (1, 1));
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('l'))]);
        assert_eq!(app.cursor_position(), (2, 1));
    }

    #[test]
    fn same_buffer_splits_share_edits() {
        let mut app = app("alpha\nbeta\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        feed(&mut app, &[key(KeyCode::Char('x'))]);
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        assert_eq!(panes.len(), 2);
        assert!(panes.iter().all(|pane| pane.lines[0] == "lpha"));
    }

    #[test]
    fn bracket_match_cache_tracks_cursor_and_edit() {
        let mut app = app("a (b) c\n");
        app.editor.apply(Action::JumpTo(0, 2));
        app.refresh_bracket_match();
        let first = app
            .windows
            .active_view()
            .bracket_match
            .expect("bracket cache");
        assert!(first.matched.is_some());
        app.editor.apply(Action::JumpTo(0, 0));
        app.refresh_bracket_match();
        let moved = app
            .windows
            .active_view()
            .bracket_match
            .expect("bracket cache");
        assert!(moved.matched.is_none());
        assert_ne!(moved.key.col, first.key.col);
        app.editor.apply(Action::JumpTo(0, 2));
        app.refresh_bracket_match();
        let revision = app
            .windows
            .active_view()
            .bracket_match
            .expect("bracket cache")
            .key
            .revision;
        app.editor.apply(Action::EnterInsert);
        app.editor.apply(Action::InsertChar('x'));
        app.refresh_bracket_match();
        let edited = app
            .windows
            .active_view()
            .bracket_match
            .expect("bracket cache");
        assert!(edited.key.revision > revision);
    }

    #[test]
    fn bracket_match_cache_is_per_window() {
        let mut app = app("a (b) c\n");
        app.editor.apply(Action::JumpTo(0, 2));
        app.refresh_bracket_match();
        let first = app.windows.active();
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        let second = app.windows.active();
        assert_ne!(first, second);
        app.editor.apply(Action::JumpTo(0, 0));
        app.refresh_bracket_match();
        assert!(app
            .windows
            .view(first)
            .and_then(|view| view.bracket_match)
            .is_some_and(|cache| cache.matched.is_some()));
        assert!(app
            .windows
            .active_view()
            .bracket_match
            .is_some_and(|cache| cache.matched.is_none()));
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        assert!(panes.iter().all(|pane| pane.bracket_match.is_none()));
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('h'))]);
        app.refresh_bracket_match();
        assert!(app
            .windows
            .active_view()
            .bracket_match
            .is_some_and(|cache| cache.matched.is_some()));
        let layout = app.window_layout();
        let panes = app.frame_panes(&layout);
        assert!(panes
            .iter()
            .find(|pane| pane.active)
            .is_some_and(|pane| pane.bracket_match.is_some()));
        assert!(panes
            .iter()
            .filter(|pane| !pane.active)
            .all(|pane| pane.bracket_match.is_none()));
    }

    #[test]
    fn bracket_match_cache_honors_highlighting_and_auto_pairs() {
        let mut app = app("a (b) c\n");
        app.editor.apply(Action::JumpTo(0, 2));
        app.editor.set_match_brackets(false);
        app.refresh_bracket_match();
        assert!(app
            .windows
            .active_view()
            .bracket_match
            .is_some_and(|cache| cache.matched.is_none()));
        app.editor.set_match_brackets(true);
        app.editor.set_auto_pairs(false);
        app.refresh_bracket_match();
        assert!(app
            .windows
            .active_view()
            .bracket_match
            .is_some_and(|cache| cache.matched.is_some()));
    }

    #[test]
    fn split_file_opens_it_in_the_new_window() {
        let path = std::env::temp_dir().join(format!(
            "kawi_window_split_{}_{}.txt",
            std::process::id(),
            "file"
        ));
        std::fs::write(&path, "split file\n").unwrap();
        let mut app = app("original\n");
        command(&mut app, &format!("vsplit {}", path.display()));
        assert_eq!(app.windows.len(), 2);
        assert_eq!(app.editor.filename(), path.to_string_lossy());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["split file"]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn q_closes_a_split_before_quitting() {
        let mut app = app("alpha\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('s'))]);
        command(&mut app, "q");
        assert_eq!(app.windows.len(), 1);
        assert_eq!(app.last_message(), None);
    }

    #[test]
    fn resize_and_equalize_keys_update_the_layout() {
        let mut app = app("alpha\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        let before = app.window_layout().panes[0].1.width;
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('>'))]);
        let after = app.window_layout().panes[0].1.width;
        assert!(after < before);
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('='))]);
        assert_eq!(app.window_layout().panes[0].1.width, before);
    }

    #[test]
    fn close_last_window_reports_e444() {
        let mut app = app("alpha\n");
        command(&mut app, "close");
        assert_eq!(app.last_message(), Some("E444: Cannot close last window"));
        assert_eq!(app.windows.len(), 1);
    }

    #[test]
    fn close_refuses_a_dirty_buffer_until_forced() {
        let mut app = app("alpha\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('s'))]);
        feed(&mut app, &[key(KeyCode::Char('x'))]);
        command(&mut app, "close");
        assert_eq!(app.windows.len(), 2);
        assert!(app
            .last_message()
            .is_some_and(|message| message.starts_with("E37:")));
        command(&mut app, "q!");
        assert_eq!(app.windows.len(), 1);
    }

    #[test]
    fn deleting_a_buffer_replaces_windows_that_show_it() {
        let path = std::env::temp_dir().join(format!(
            "kawi_window_delete_{}_{}.txt",
            std::process::id(),
            "other"
        ));
        std::fs::write(&path, "other\n").unwrap();
        let mut app = app("original\n");
        command(&mut app, &format!("vsplit {}", path.to_string_lossy()));
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('h'))]);
        command(&mut app, "bd other");
        assert_eq!(app.windows.len(), 2);
        assert_eq!(app.windows.count_buffer(1), 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn only_closes_other_windows() {
        let mut app = app("alpha\n");
        feed(
            &mut app,
            &[
                ctrl_w(),
                key(KeyCode::Char('s')),
                ctrl_w(),
                key(KeyCode::Char('v')),
            ],
        );
        assert_eq!(app.windows.len(), 3);
        command(&mut app, "only");
        assert_eq!(app.windows.len(), 1);
    }

    #[test]
    fn mouse_click_focuses_the_pane_under_the_pointer() {
        let mut app = app("alpha\nbeta\ngamma\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        let right = app.windows.active();
        app.handle_mouse(press(1, 2)).expect("click");
        assert_ne!(app.windows.active(), right);
        assert_eq!(app.cursor_position(), (2, 0));
    }

    #[test]
    fn mouse_drag_only_edits_the_active_pane() {
        let mut app = app("alpha\nbeta\n");
        feed(&mut app, &[ctrl_w(), key(KeyCode::Char('v'))]);
        let active = app.windows.active();
        let layout = app.window_layout();
        let other = layout
            .panes
            .iter()
            .find(|(id, _)| *id != active)
            .map(|(_, rect)| *rect)
            .expect("inactive pane");
        app.handle_mouse(drag(
            u16::try_from(other.x.saturating_add(1)).unwrap(),
            u16::try_from(other.y.saturating_add(1)).unwrap(),
        ))
        .expect("drag inactive");
        assert_eq!(app.editor_mode(), kawi_editor::Mode::Normal);
        let current = layout
            .panes
            .iter()
            .find(|(id, _)| *id == active)
            .map(|(_, rect)| *rect)
            .expect("active pane");
        app.handle_mouse(drag(
            u16::try_from(current.x.saturating_add(1)).unwrap(),
            u16::try_from(current.y.saturating_add(1)).unwrap(),
        ))
        .expect("drag active");
        assert_eq!(app.editor_mode(), kawi_editor::Mode::Visual);
    }

    #[test]
    fn window_prefix_is_shown_while_armed() {
        let mut app = app("alpha\n");
        app.handle_key(ctrl_w()).expect("prefix");
        assert_eq!(app.showcmd_text().as_deref(), Some("^W"));
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert_eq!(app.showcmd_text(), None);
    }

    #[test]
    fn starts_with_one_window() {
        let windows = windows(1);
        let layout = windows.layout(area());
        assert_eq!(windows.len(), 1);
        assert_eq!(layout.panes, vec![(1, area())]);
        assert!(layout.separators.is_empty());
    }

    #[test]
    fn nested_splits_share_the_available_area() {
        let mut windows = Windows::new(1, VIEW);
        let right = windows.split_active(SplitDirection::Vertical, WindowView::new(1, VIEW));
        let bottom = windows.split_active(SplitDirection::Horizontal, WindowView::new(1, VIEW));
        let layout = windows.layout(area());
        assert_eq!(layout.panes.len(), 3);
        assert_eq!(layout.separators.len(), 1);
        let left_rect = pane_rect(&layout.panes, 1).expect("left pane");
        let right_rect = pane_rect(&layout.panes, right).expect("right pane");
        let bottom_rect = pane_rect(&layout.panes, bottom).expect("bottom pane");
        assert_eq!(
            (left_rect.x, left_rect.y, left_rect.width, left_rect.height),
            (0, 0, 50, 40)
        );
        assert_eq!(
            (
                right_rect.x,
                right_rect.y,
                right_rect.width,
                right_rect.height
            ),
            (51, 0, 49, 20)
        );
        assert_eq!(
            (
                bottom_rect.x,
                bottom_rect.y,
                bottom_rect.width,
                bottom_rect.height
            ),
            (51, 20, 49, 20)
        );
    }

    #[test]
    fn horizontal_splits_stack_without_separators() {
        let mut windows = windows(1);
        let second = windows.split_active(SplitDirection::Horizontal, WindowView::new(2, VIEW));
        let layout = windows.layout(area());
        assert_eq!(windows.active(), second);
        assert_eq!(layout.panes.len(), 2);
        assert_eq!(layout.panes[0].1, Rect::new(0, 0, 100, 20));
        assert_eq!(layout.panes[1].1, Rect::new(0, 20, 100, 20));
        assert!(layout.separators.is_empty());
    }

    #[test]
    fn vertical_splits_reserve_a_separator_column() {
        let mut windows = windows(1);
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, VIEW));
        let layout = windows.layout(area());
        assert_eq!(layout.panes[0].1, Rect::new(0, 0, 50, 40));
        assert_eq!(layout.panes[1].1, Rect::new(51, 0, 49, 40));
        assert_eq!(layout.separators, vec![Rect::new(50, 0, 1, 40)]);
    }

    #[test]
    fn removes_a_window_and_focuses_its_sibling() {
        let mut windows = windows(2);
        let first = 1;
        assert_eq!(windows.active(), 2);
        assert_eq!(windows.remove_active(), Some(first));
        assert_eq!(windows.len(), 1);
        assert_eq!(windows.active(), first);
        assert!(windows.view(2).is_none());
    }

    #[test]
    fn refuses_to_remove_the_last_window() {
        let mut windows = windows(1);
        assert_eq!(windows.remove_active(), None);
        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn only_keeps_the_active_window() {
        let mut windows = windows(3);
        assert_eq!(windows.only_active(), 2);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows.layout(area()).panes.len(), 1);
    }

    #[test]
    fn focus_moves_by_geometry() {
        let mut windows = windows(1);
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, VIEW));
        let left = 1;
        assert_eq!(windows.active(), 2);
        assert_eq!(
            windows.focus_direction(FocusDirection::Left, area()),
            Some(left)
        );
        assert!(windows.set_active(left));
        assert_eq!(
            windows.focus_direction(FocusDirection::Right, area()),
            Some(2)
        );
        assert_eq!(windows.focus_direction(FocusDirection::Down, area()), None);
    }

    #[test]
    fn focus_cycles_in_layout_order() {
        let windows = windows(3);
        assert_eq!(windows.focus_next(area(), false), Some(1));
        assert_eq!(windows.focus_next(area(), true), Some(2));
        assert_eq!(windows.active(), 3);
    }

    #[test]
    fn resize_adjusts_the_nearest_matching_split() {
        let mut windows = windows(1);
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, VIEW));
        assert!(windows.resize_active(ResizeAxis::Width, false));
        let layout = windows.layout(area());
        assert!(layout.panes[0].1.width > 50);
        assert!(windows.resize_active(ResizeAxis::Width, true));
        assert_eq!(windows.layout(area()).panes[0].1.width, 50);
    }

    #[test]
    fn equalize_resets_every_split() {
        let mut windows = windows(1);
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, VIEW));
        windows.resize_active(ResizeAxis::Width, true);
        assert!(windows.equalize());
        assert_eq!(windows.layout(area()).panes[0].1.width, 50);
    }

    #[test]
    fn tiny_areas_still_produce_panes() {
        let mut windows = windows(1);
        windows.split_active(SplitDirection::Vertical, WindowView::new(2, VIEW));
        let layout = windows.layout(Rect::new(0, 0, 3, 2));
        assert_eq!(layout.panes.len(), 2);
        assert!(layout.panes.iter().all(|(_, rect)| rect.width > 0));
    }
}
