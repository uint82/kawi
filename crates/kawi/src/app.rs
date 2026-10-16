use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyEvent, MouseEvent};
use kawi_buffer::Buffer;
use std::io::{stdout, Stdout};

use crate::keymap::{flavor_from_name, ActiveKeymap, Flavor, Keymap};
use crate::mouse::{drag_position, press_position, wheel_delta};
use kawi_editor::{parse, Action, Command, Editor, MatchSpan, SearchOutcome};
use kawi_tui::{
    frame::{Frame, SearchSpan},
    render,
    statusbar::StatusInfo,
    terminal,
    theme::Theme,
    viewport::Viewport,
};

pub struct App {
    editor: Editor,
    viewport: Viewport,
    keymap: ActiveKeymap,
    theme: Theme,
    quit_armed: bool,
    message: Option<String>,
}

impl App {
    pub fn new(buffer: Buffer, flavor: Flavor) -> Self {
        Self {
            editor: Editor::new(buffer),
            viewport: Viewport::new(fallback_height(), fallback_width()),
            keymap: ActiveKeymap::for_flavor(flavor),
            theme: Theme::default(),
            quit_armed: false,
            message: None,
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

    #[cfg(test)]
    pub fn last_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    #[cfg(test)]
    pub fn search_status_line(&self) -> Option<String> {
        self.editor.search_status_text()
    }

    #[cfg(test)]
    pub fn theme_name(&self) -> &str {
        self.theme.name
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
        let window = self
            .editor
            .visible_lines(self.viewport.top(), self.viewport.height());
        let search_status = self.editor.search_status_text();
        let search_spans = frame_search_spans(
            &self
                .editor
                .search_spans(self.viewport.top(), self.viewport.height()),
        );
        let frame = self.build_frame(&status, &window, search_status.as_deref(), &search_spans);
        render::draw(out, &frame)
    }

    fn build_frame<'a>(
        &'a self,
        status: &'a StatusInfo<'a>,
        window: &'a [String],
        search_status: Option<&'a str>,
        search_spans: &'a [SearchSpan],
    ) -> Frame<'a> {
        Frame {
            lines: window,
            col: self.editor.col(),
            row: self.editor.row(),
            viewport: &self.viewport,
            status,
            message: self.message.as_deref(),
            command: self.editor.command_prompt(),
            search: self.editor.search_prompt(),
            search_status,
            search_spans,
            showcmd: self.keymap.pending_text(),
            insert_mode: self.editor.mode().is_insert(),
            selection: self.editor.selection_range(),
            theme: &self.theme,
        }
    }

    fn show_message(&mut self, text: String) {
        self.message = Some(text);
    }

    fn clear_message(&mut self) {
        self.message = None;
    }

    fn status_snapshot(&self) -> StatusInfo<'_> {
        StatusInfo {
            filename: self.editor.filename(),
            dirty: self.editor.is_dirty(),
            row: self.editor.row(),
            col: self.editor.col(),
            quit_armed: self.quit_armed,
            mode: self.keymap.mode_label(self.editor.mode()),
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
        self.viewport.scroll_lines(delta, self.editor.line_count());
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
            Action::Undo => self.handle_undo(),
            Action::Redo => self.handle_redo(),
            Action::NextMatch => self.handle_next_match(),
            Action::PrevMatch => self.handle_prev_match(),
            Action::ConfirmSearch => self.handle_confirm_search(),
            Action::Noop => self.handle_noop(),
            _ => self.handle_engine_action(action),
        }
    }

    fn handle_save(&mut self) -> Result<bool> {
        self.disarm_quit();
        self.save_and_report();
        Ok(false)
    }

    fn save_and_report(&mut self) -> bool {
        match self.editor.save() {
            Ok(()) => self.report_saved(),
            Err(error) => self.report_error(error.to_string()),
        }
    }

    fn report_error(&mut self, text: String) -> bool {
        self.show_message(text);
        false
    }

    fn report_saved(&mut self) -> bool {
        let report = save_report(
            self.editor.filename(),
            self.editor.line_count(),
            self.editor.byte_count(),
        );
        self.show_message(report);
        true
    }

    fn handle_undo(&mut self) -> Result<bool> {
        self.disarm_quit();
        if !self.editor.try_undo() {
            self.show_message("Already at oldest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_redo(&mut self) -> Result<bool> {
        self.disarm_quit();
        if !self.editor.try_redo() {
            self.show_message("Already at newest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_next_match(&mut self) -> Result<bool> {
        self.disarm_quit();
        self.report_search_step(true);
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_prev_match(&mut self) -> Result<bool> {
        self.disarm_quit();
        self.report_search_step(false);
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_confirm_search(&mut self) -> Result<bool> {
        self.disarm_quit();
        let outcome = self.editor.confirm_search();
        self.report_outcome(outcome, true);
        self.reveal_cursor();
        Ok(false)
    }

    fn report_search_step(&mut self, forward: bool) {
        let outcome = search_step(&mut self.editor, forward);
        self.report_outcome(Some(outcome), forward);
    }

    fn report_outcome(&mut self, outcome: Option<SearchOutcome>, forward: bool) {
        match outcome {
            None => {}
            Some(SearchOutcome::Found { wrapped: false }) => self.clear_message(),
            Some(SearchOutcome::Found { wrapped: true }) => {
                self.show_message(wrap_message(forward))
            }
            Some(SearchOutcome::Missing) => self.show_message(missing_message(&self.editor)),
        }
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
            Command::SetTheme(name) => self.switch_theme(&name),
            Command::SetShiftWidth(width) => self.switch_shiftwidth(width),
            Command::SetExpandTab(expandtab) => self.switch_expandtab(expandtab),
            Command::Unknown => Ok(false),
        }
    }

    fn switch_keymap(&mut self, name: &str) -> Result<bool> {
        match flavor_from_name(name) {
            Some(flavor) => self.apply_flavor(flavor),
            None => self.report_unknown_keymap(name),
        }
    }

    fn report_unknown_keymap(&mut self, name: &str) -> Result<bool> {
        self.show_message(format!("Unknown keymap: {name}"));
        Ok(false)
    }

    fn switch_shiftwidth(&mut self, width: usize) -> Result<bool> {
        self.editor.set_shiftwidth(width);
        Ok(false)
    }

    fn switch_expandtab(&mut self, expandtab: bool) -> Result<bool> {
        self.editor.set_expandtab(expandtab);
        Ok(false)
    }

    fn switch_theme(&mut self, name: &str) -> Result<bool> {
        match Theme::named(name) {
            Some(theme) => self.apply_theme(theme),
            None => self.report_unknown_theme(name),
        }
    }

    fn report_unknown_theme(&mut self, name: &str) -> Result<bool> {
        self.show_message(format!("Unknown theme: {name}"));
        Ok(false)
    }

    fn apply_theme(&mut self, theme: Theme) -> Result<bool> {
        self.theme = theme;
        Ok(false)
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
        if self.save_and_report() {
            return Ok(self.try_quit());
        }
        Ok(false)
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

fn frame_search_spans(spans: &[MatchSpan]) -> Vec<SearchSpan> {
    spans.iter().map(frame_search_span).collect()
}

fn frame_search_span(span: &MatchSpan) -> SearchSpan {
    SearchSpan {
        row: span.row,
        start: span.start,
        end: span.end,
        current: span.current,
    }
}

fn search_step(editor: &mut Editor, forward: bool) -> SearchOutcome {
    if forward {
        editor.search_next()
    } else {
        editor.search_prev()
    }
}

fn wrap_message(forward: bool) -> String {
    if forward {
        return "search hit BOTTOM, continuing at TOP".to_string();
    }
    "search hit TOP, continuing at BOTTOM".to_string()
}

fn missing_message(editor: &Editor) -> String {
    if editor.search_pattern().is_empty() {
        return "E35: No previous regular expression".to_string();
    }
    format!("E486: Pattern not found: {}", editor.search_pattern())
}

fn save_report(filename: &str, line_count: usize, byte_count: usize) -> String {
    format!("\"{filename}\" {line_count}L, {byte_count}B written")
}

fn is_operator(action: Action) -> bool {
    matches!(
        action,
        Action::DeleteSelection
            | Action::YankSelection
            | Action::PasteOverSelection
            | Action::DeleteLines
            | Action::YankLines
            | Action::ShiftRight
            | Action::ShiftLeft
            | Action::Reindent
            | Action::ToggleCase
            | Action::Lowercase
            | Action::Uppercase
    )
}

fn content_size(cols: u16, rows: u16) -> (usize, usize) {
    let width = usize::from(cols).max(1);
    let height = usize::from(rows).saturating_sub(2).max(1);
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
    // NOTE(probes): nvim verification lessons. Run one insert entry per
    // `:normal` call because later entries are dropped, end insert-ending
    // sequences with a real Esc because stopinsert also moves the cursor
    // left, and keep every motion succeeding because :normal aborts the
    // rest of the keys after a failed one.
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
    fn vim_visual_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('c'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_capital_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('C'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_capital_c_enters_insert() {
        let mut app = App::new(Buffer::from_string("abc\ndef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('C'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_capital_s_enters_insert() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('S'))).expect("substitute");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_capital_s_enters_insert() {
        let mut app = App::new(Buffer::from_string("abc\ndef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('S'))).expect("substitute");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_line_toggle() {
        let mut app = App::new(Buffer::from_string("a\nb\nc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        assert_eq!(app.editor_mode(), Mode::VisualLine);
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('V'))).expect("exit");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn vim_visual_capital_d_deletes_lines() {
        let mut app = App::new(Buffer::from_string("aBcDeF"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('D'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_tilde_toggles() {
        let mut app = App::new(Buffer::from_string("aBcDeF"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('~'))).expect("toggle");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_tilde_w_toggles_word() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('~'))).expect("tilde");
        app.handle_key(key(KeyCode::Char('w'))).expect("word");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_lower_line_toggles_line() {
        let mut app = App::new(Buffer::from_string("foo BAR"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('u'))).expect("lower");
        app.handle_key(key(KeyCode::Char('u'))).expect("line");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_goto_last_line() {
        let mut app = App::new(Buffer::from_string("abcdef\ngh"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (1, 1));
    }

    #[test]
    fn vim_normal_goto_first_line() {
        let mut app = App::new(Buffer::from_string("ab\ncd\nef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_goto_extends() {
        let mut app = App::new(Buffer::from_string("ab\ncd\nef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (2, 0))));
        assert_eq!(app.cursor_position(), (2, 0));
    }

    #[test]
    fn vim_normal_delete_to_last_line() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_delete_to_end_then_j_leaves_no_selection() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc\ndd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('G'))).expect("goto");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_dgg_from_last() {
        let mut app = App::new(Buffer::from_string("a\nbb\ncc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('g'))).expect("goto");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_gv_reselects() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('v'))).expect("exit");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('v'))).expect("reselect");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_visual_gv_exchanges() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('v'))).expect("exit");
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('h'))).expect("shrink");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('v'))).expect("exchange");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_normal_gi_resumes_insert() {
        let mut app = App::new(Buffer::from_string("ab"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('i'))).expect("insert");
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        app.handle_key(key(KeyCode::Char('h'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('i'))).expect("resume");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.cursor_position(), (0, 1));
        app.handle_key(key(KeyCode::Char('Z'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 1));
    }

    #[test]
    fn vim_normal_shift_right() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_shift_right_exits() {
        let mut app = App::new(Buffer::from_string("ab\ncd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_visual_reindent_exits() {
        let mut app = App::new(Buffer::from_string("    a\n    b"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('V'))).expect("linewise");
        app.handle_key(key(KeyCode::Char('j'))).expect("extend");
        app.handle_key(key(KeyCode::Char('='))).expect("reindent");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_shift_word() {
        let mut app = App::new(Buffer::from_string("foo BAR"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('w'))).expect("word");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_normal_shift_find() {
        let mut app = App::new(Buffer::from_string("axbxc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('>'))).expect("shift");
        app.handle_key(key(KeyCode::Char('f'))).expect("find");
        app.handle_key(key(KeyCode::Char('x'))).expect("target");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_dot_repeats_delete() {
        let mut app = App::new(Buffer::from_string("a\nb\nc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('d'))).expect("line");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('.'))).expect("repeat");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 0));
    }

    #[test]
    fn vim_dot_repeats_change_text() {
        let mut app = App::new(Buffer::from_string("aaa\nbbb"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('c'))).expect("change");
        app.handle_key(key(KeyCode::Char('c'))).expect("line");
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        app.handle_key(key(KeyCode::Char('j'))).expect("move");
        app.handle_key(key(KeyCode::Char('.'))).expect("repeat");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (1, 0));
    }

    #[test]
    fn vim_visual_o_swaps_ends() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('v'))).expect("visual");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Char('o'))).expect("swap");
        assert_eq!(app.editor_mode(), Mode::Visual);
        assert_eq!(app.cursor_position(), (0, 0));
        assert_eq!(app.editor_selection(), Some(((0, 0), (0, 2))));
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
    fn helix_esc_returns_to_normal() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('v'))).expect("select");
        app.handle_key(key(KeyCode::Char('l'))).expect("extend");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.editor_selection(), None);
    }

    #[test]
    fn helix_delete_acts_on_cursor_char() {
        let mut app = App::new(Buffer::from_string("abcdef"), Flavor::Helix);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 1));
    }

    #[test]
    fn change_word_enters_insert() {
        let mut app = App::new(Buffer::from_string("foo bar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('c'))).expect("pending");
        app.handle_key(key(KeyCode::Char('w'))).expect("change");
        assert_eq!(app.editor_mode(), Mode::Insert);
    }

    #[test]
    fn save_reports_lines_and_bytes() {
        let path = std::env::temp_dir().join("kawi_msg_test.txt");
        let mut buffer = Buffer::from_string("aa\nbb");
        buffer.path = Some(path.to_string_lossy().to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.handle_save().expect("save");
        let expected = format!("\"{}\" 2L, 5B written", path.to_string_lossy());
        assert_eq!(app.last_message(), Some(expected.as_str()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_without_path_reports_error() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_save().expect("stays");
        assert_eq!(app.last_message(), Some("E32: No file name"));
    }

    #[test]
    fn undo_without_history_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_undo().expect("undo");
        assert_eq!(app.last_message(), Some("Already at oldest change"));
    }

    #[test]
    fn redo_without_future_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.handle_redo().expect("redo");
        assert_eq!(app.last_message(), Some("Already at newest change"));
    }

    #[test]
    fn unknown_keymap_reports() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.run_command(Command::SetKeymap("personal".to_string()))
            .expect("unknown");
        assert_eq!(app.last_message(), Some("Unknown keymap: personal"));
    }

    #[test]
    fn save_report_formats_like_editor() {
        assert_eq!(
            save_report("notes.txt", 3, 12),
            "\"notes.txt\" 3L, 12B written"
        );
    }

    #[test]
    fn search_prompt_types_and_confirms() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        assert_eq!(app.editor_mode(), Mode::Search);
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        assert_eq!(app.cursor_position(), (0, 5));
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 5));
    }

    #[test]
    fn search_cancel_restores_origin() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("cancel");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 4));
    }

    #[test]
    fn search_next_reports_wrap() {
        let mut app = App::new(Buffer::from_string("ar ar ar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.cursor_position(), (0, 3));
        assert_eq!(app.last_message(), None);
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        app.handle_key(key(KeyCode::Char('n'))).expect("wrap");
        assert_eq!(app.cursor_position(), (0, 0));
        assert_eq!(
            app.last_message(),
            Some("search hit BOTTOM, continuing at TOP")
        );
    }

    #[test]
    fn search_without_pattern_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(
            app.last_message(),
            Some("E35: No previous regular expression")
        );
    }

    #[test]
    fn search_no_match_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('z'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.last_message(), Some("E486: Pattern not found: z"));
    }

    #[test]
    fn search_confirm_keeps_status_without_message() {
        let mut app = App::new(Buffer::from_string("foo bar foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 5));
        assert_eq!(app.last_message(), None);
        assert_eq!(app.search_status_line().as_deref(), Some("/ar [1/1]"));
    }

    #[test]
    fn search_confirm_without_match_reports() {
        let mut app = App::new(Buffer::from_string("foo"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('z'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.last_message(), Some("E486: Pattern not found: z"));
        assert_eq!(app.search_status_line(), None);
    }

    #[test]
    fn set_theme_switches_palette() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        assert_eq!(app.theme_name(), "default");
        app.run_command(Command::SetTheme("gruvbox-dark".to_string()))
            .expect("switch");
        assert_eq!(app.theme_name(), "gruvbox-dark");
        assert_eq!(app.last_message(), None);
    }

    #[test]
    fn set_theme_unknown_reports_and_keeps() {
        let mut app = App::new(Buffer::from_string("hi"), Flavor::Vim);
        app.run_command(Command::SetTheme("nope".to_string()))
            .expect("unknown");
        assert_eq!(app.theme_name(), "default");
        assert_eq!(app.last_message(), Some("Unknown theme: nope"));
    }

    #[test]
    fn search_next_restores_status_after_wrap() {
        let mut app = App::new(Buffer::from_string("ar ar ar"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        app.handle_key(key(KeyCode::Char('a'))).expect("type");
        app.handle_key(key(KeyCode::Char('r'))).expect("type");
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        app.handle_key(key(KeyCode::Char('n'))).expect("wrap");
        assert_eq!(
            app.last_message(),
            Some("search hit BOTTOM, continuing at TOP")
        );
        app.handle_key(key(KeyCode::Char('n'))).expect("next");
        assert_eq!(app.last_message(), None);
        assert_eq!(app.cursor_position(), (0, 3));
        assert_eq!(app.search_status_line().as_deref(), Some("/ar [2/3]"));
    }
}
