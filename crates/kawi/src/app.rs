use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyEvent, MouseEvent};
use kawi_buffer::Buffer;
use std::io::{stdout, Stdout};

use crate::keymap::{flavor_from_name, ActiveKeymap, Flavor, Keymap};
use crate::mouse::{drag_position, press_position, wheel_delta};
use kawi_editor::{parse, Action, Command, Editor};
use kawi_tui::{frame::Frame, render, statusbar::StatusInfo, terminal, viewport::Viewport};

pub struct App {
    editor: Editor,
    viewport: Viewport,
    keymap: ActiveKeymap,
    quit_armed: bool,
}

impl App {
    pub fn new(buffer: Buffer, flavor: Flavor) -> Self {
        Self {
            editor: Editor::new(buffer),
            viewport: Viewport::new(fallback_height(), fallback_width()),
            keymap: ActiveKeymap::for_flavor(flavor),
            quit_armed: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.sync_viewport_size();
        self.run_loop()
    }

    #[cfg(test)]
    pub fn editor_mode(&self) -> kawi_editor::Mode {
        self.editor.mode()
    }

    #[cfg(test)]
    pub fn editor_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.editor.selection_range()
    }

    #[cfg(test)]
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.editor.row(), self.editor.col())
    }

    #[cfg(test)]
    pub fn viewport_top(&self) -> usize {
        self.viewport.top()
    }

    fn run_loop(&mut self) -> Result<()> {
        let mut out = stdout();
        loop {
            self.draw_frame(&mut out)?;
            let event = read_next_event()?;
            if self.update(event)? {
                break;
            }
        }
        Ok(())
    }

    fn draw_frame(&self, out: &mut Stdout) -> Result<()> {
        let status = self.status_snapshot();
        let frame = self.build_frame(&status);
        render::draw(out, &frame)
    }

    fn build_frame<'a>(&'a self, status: &'a StatusInfo<'a>) -> Frame<'a> {
        Frame {
            lines: self.editor.lines(),
            col: self.editor.col(),
            row: self.editor.row(),
            viewport: &self.viewport,
            status,
            insert_mode: self.editor.mode().is_insert(),
            selection: self.editor.selection_range(),
        }
    }

    fn status_snapshot(&self) -> StatusInfo<'_> {
        StatusInfo {
            filename: self.editor.filename(),
            dirty: self.editor.is_dirty(),
            row: self.editor.row(),
            col: self.editor.col(),
            quit_armed: self.quit_armed,
            mode: self.keymap.mode_label(self.editor.mode()),
            command: self.editor.command_prompt(),
        }
    }

    fn update(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(cols, rows) => self.handle_resize(cols, rows),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            _ => Ok(false),
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<bool> {
        if let Some((row, col)) = press_position(&mouse, &self.viewport) {
            return self.jump_to(row, col);
        }
        if let Some((row, col)) = drag_position(&mouse, &self.viewport) {
            return self.drag_to(row, col);
        }
        if let Some(delta) = wheel_delta(&mouse) {
            return self.scroll_by(delta);
        }
        self.handle_noop()
    }

    fn jump_to(&mut self, row: usize, col: usize) -> Result<bool> {
        self.disarm_quit();
        self.exit_visual_if_active();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
    }

    fn exit_visual_if_active(&mut self) {
        if self.editor.mode().is_visual() {
            self.editor.apply(Action::ExitVisual);
        }
    }

    fn drag_to(&mut self, row: usize, col: usize) -> Result<bool> {
        self.disarm_quit();
        self.ensure_visual();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
    }

    fn scroll_by(&mut self, delta: isize) -> Result<bool> {
        self.disarm_quit();
        self.viewport.scroll_lines(delta, self.editor.lines().len());
        self.follow_cursor_into_view();
        Ok(false)
    }

    fn follow_cursor_into_view(&mut self) {
        let row = self.editor.row().clamp(
            self.viewport.top(),
            self.viewport.visible_end().saturating_sub(1),
        );
        if row != self.editor.row() {
            self.editor.apply(Action::JumpTo(row, self.editor.col()));
        }
    }

    fn ensure_visual(&mut self) {
        if !self.editor.mode().is_visual() {
            self.editor.apply(Action::EnterVisual);
        }
    }

    fn handle_resize(&mut self, cols: u16, rows: u16) -> Result<bool> {
        self.set_viewport_size(cols, rows);
        self.reveal_cursor();
        Ok(false)
    }

    fn set_viewport_size(&mut self, cols: u16, rows: u16) {
        let (width, height) = content_size(cols, rows);
        self.viewport.set_size(width, height);
    }

    fn sync_viewport_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.set_viewport_size(cols, rows);
        }
    }

    fn reveal_cursor(&mut self) {
        self.viewport.ensure_visible(self.editor.row());
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        let action = self.keymap.map(key, self.editor.mode());
        self.dispatch_action(action)
    }

    fn dispatch_action(&mut self, action: Action) -> Result<bool> {
        match action {
            Action::Quit => Ok(self.try_quit()),
            Action::Save => self.handle_save(),
            Action::ExecuteCommand => self.handle_execute(),
            Action::Noop => self.handle_noop(),
            _ => self.handle_engine_action(action),
        }
    }

    fn handle_save(&mut self) -> Result<bool> {
        self.disarm_quit();
        self.editor.save()?;
        Ok(false)
    }

    fn handle_noop(&mut self) -> Result<bool> {
        self.disarm_quit();
        Ok(false)
    }

    fn handle_engine_action(&mut self, action: Action) -> Result<bool> {
        self.disarm_quit();
        self.editor.apply(action);
        self.exit_operator_for_flavor(action);
        self.reveal_cursor();
        Ok(false)
    }

    fn exit_operator_for_flavor(&mut self, action: Action) {
        if !self.keymap.operator_exits_visual() {
            return;
        }
        if !is_operator(action) {
            return;
        }
        self.editor.apply(Action::ExitVisual);
    }

    fn handle_execute(&mut self) -> Result<bool> {
        self.disarm_quit();
        let command = parse(self.editor.command_text());
        self.editor.apply(Action::CancelCommand);
        self.run_command(command)
    }

    fn run_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Write => self.handle_save(),
            Command::Quit => Ok(self.try_quit()),
            Command::WriteQuit => self.handle_write_quit(),
            Command::SetKeymap(name) => self.switch_keymap(&name),
            Command::Unknown => Ok(false),
        }
    }

    fn switch_keymap(&mut self, name: &str) -> Result<bool> {
        match flavor_from_name(name) {
            Some(flavor) => self.apply_flavor(flavor),
            None => Ok(false),
        }
    }

    fn apply_flavor(&mut self, flavor: Flavor) -> Result<bool> {
        self.keymap = ActiveKeymap::for_flavor(flavor);
        if flavor == Flavor::Vim {
            self.exit_visual_if_active();
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_write_quit(&mut self) -> Result<bool> {
        self.editor.save()?;
        Ok(self.try_quit())
    }

    fn try_quit(&mut self) -> bool {
        if !self.editor.is_dirty() {
            return true;
        }
        self.confirm_dirty_quit()
    }

    fn confirm_dirty_quit(&mut self) -> bool {
        if self.quit_armed {
            return true;
        }
        self.arm_quit();
        false
    }

    fn arm_quit(&mut self) {
        self.quit_armed = true;
    }

    fn disarm_quit(&mut self) {
        self.quit_armed = false;
    }
}

fn read_next_event() -> Result<Event> {
    read().context("read terminal event")
}

fn is_operator(action: Action) -> bool {
    matches!(action, Action::DeleteSelection | Action::YankSelection)
}

fn content_size(cols: u16, rows: u16) -> (usize, usize) {
    let width = usize::from(cols).max(1);
    let height = usize::from(rows).saturating_sub(1).max(1);
    (width, height)
}

fn fallback_width() -> usize {
    80
}

fn fallback_height() -> usize {
    24
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
    use kawi_editor::Mode;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
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

    fn wheel_down() -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn drag_selects_from_press_point() {
        let mut app = App::new(Buffer::from_string("abcdef\nghijkl"), Flavor::Vim);
        app.handle_mouse(press(1, 0)).expect("press");
        app.handle_mouse(drag(4, 1)).expect("drag");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 1), (1, 4))));
    }

    #[test]
    fn press_after_drag_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef\nghijkl"), Flavor::Vim);
        app.handle_mouse(press(1, 0)).expect("press");
        app.handle_mouse(drag(4, 1)).expect("drag");
        app.handle_mouse(press(0, 0)).expect("second press");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn wheel_down_scrolls_and_drags_cursor() {
        let mut app = App::new(Buffer::from_string(&"line\n".repeat(30)), Flavor::Vim);
        app.handle_mouse(wheel_down()).expect("wheel");
        assert_eq!(app.viewport_top(), 3);
        assert_eq!(app.cursor_position(), (3, 0));
    }

    #[test]
    fn wheel_keeps_visible_cursor() {
        let mut app = App::new(Buffer::from_string(&"line\n".repeat(30)), Flavor::Vim);
        app.handle_mouse(press(0, 10)).expect("press");
        app.handle_mouse(wheel_down()).expect("wheel");
        assert_eq!(app.viewport_top(), 3);
        assert_eq!(app.cursor_position(), (10, 0));
    }

    #[test]
    fn vim_delete_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_yank_leaves_visual() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('y'))).expect("yank");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn helix_starts_clean() {
        let app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_switches_to_helix() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.run_command(Command::SetKeymap("helix".to_string()))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_back_to_vim_clears() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.run_command(Command::SetKeymap("vim".to_string()))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn set_keymap_helix_preserves_range() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.run_command(Command::SetKeymap("helix".to_string()))
            .expect("switch");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn set_keymap_unknown_keeps_flavor() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.run_command(Command::SetKeymap("personal".to_string()))
            .expect("unknown");
        assert_eq!(app.editor_mode(), Mode::Normal);
    }

    #[test]
    fn helix_motion_moves_without_extending() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn helix_v_selects_then_motion_extends() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('v'))).expect("select");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
    }

    #[test]
    fn helix_delete_acts_on_cursor_char() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 1));
    }
}
