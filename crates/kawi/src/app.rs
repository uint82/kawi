use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
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
};

pub struct App {
    editor: Editor,
    keymap: ActiveKeymap,
    theme: Theme,
    message: Option<String>,
    pager: Option<Vec<String>>,
}

impl App {
    pub fn new(buffer: Buffer, flavor: Flavor) -> Self {
        let mut editor = Editor::new(buffer);
        editor.set_clipboard(Box::new(kawi_tui::clipboard::SystemClipboard::new()));
        Self {
            editor,
            keymap: ActiveKeymap::for_flavor(flavor),
            theme: Theme::default(),
            message: None,
            pager: None,
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
        self.editor.view_top()
    }

    #[cfg(test)]
    pub fn last_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    #[cfg(test)]
    pub fn pager_lines(&self) -> Option<&[String]> {
        self.pager.as_deref()
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
            .visible_lines(self.editor.view_top(), self.editor.view_height());
        let gutter = self
            .editor
            .gutter_lines(self.editor.view_top(), self.editor.view_height());
        let search_status = self.editor.search_status_text();
        let search_spans = frame_search_spans(
            &self
                .editor
                .search_spans(self.editor.view_top(), self.editor.view_height()),
        );
        let frame = self.build_frame(
            &status,
            &window,
            &gutter,
            search_status.as_deref(),
            &search_spans,
        );
        render::draw(out, &frame)
    }

    fn build_frame<'a>(
        &'a self,
        status: &'a StatusInfo<'a>,
        window: &'a [String],
        gutter: &'a [String],
        search_status: Option<&'a str>,
        search_spans: &'a [SearchSpan],
    ) -> Frame<'a> {
        Frame {
            lines: window,
            gutter,
            pager: self.pager.as_deref(),
            cursor_col: self.editor.cursor_wcol(),
            row: self.editor.row(),
            viewport: self.editor.viewport(),
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
        let lines = pager_lines(&text, self.editor.viewport().width());
        if lines.len() > 1 {
            self.message = None;
            self.pager = Some(lines);
        } else {
            self.pager = None;
            self.message = Some(text);
        }
    }

    fn clear_message(&mut self) {
        self.message = None;
        self.pager = None;
    }

    fn status_snapshot(&self) -> StatusInfo<'_> {
        StatusInfo {
            filename: self.editor.filename(),
            dirty: self.editor.is_dirty(),
            row: self.editor.row(),
            col: self.editor.col(),
            virtcol: self.editor.virtcol(),
            line_empty: self.editor.mode() != kawi_editor::Mode::Insert
                && self.editor.cursor_line_empty(),
            empty_buffer: self.editor.is_empty(),
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
        if self.pager.is_some() {
            return self.handle_noop();
        }
        if let Some((row, screen_col)) = press_position(&mouse, self.view_geometry()) {
            let col = self.editor.buffer_col_at_screen(row, screen_col);
            return self.jump_to(row, col);
        }
        if let Some((row, screen_col)) = drag_position(&mouse, self.view_geometry()) {
            let col = self.editor.buffer_col_at_screen(row, screen_col);
            return self.drag_to(row, col);
        }
        if let Some(delta) = wheel_delta(&mouse) {
            return self.scroll_by(delta);
        }
        self.handle_noop()
    }

    fn jump_to(&mut self, row: usize, col: usize) -> Result<bool> {
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
        self.ensure_visual();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
    }

    fn scroll_by(&mut self, delta: isize) -> Result<bool> {
        self.editor.scroll_wheel(delta);
        Ok(false)
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
        self.editor.set_view_size(width, height);
    }

    fn sync_viewport_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.set_viewport_size(cols, rows);
        }
    }

    fn view_geometry(&self) -> (usize, usize) {
        (self.editor.view_top(), self.editor.view_height())
    }

    fn reveal_cursor(&mut self) {
        self.editor.reveal_cursor();
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.pager.is_some() {
            return self.handle_pager_key(key);
        }
        let action = self.keymap.map(key, self.editor.mode());
        self.dispatch_action(action)
    }

    fn handle_pager_key(&mut self, key: KeyEvent) -> Result<bool> {
        self.pager = None;
        if is_pager_dismiss(key) {
            return Ok(false);
        }
        let action = self.keymap.map(key, self.editor.mode());
        self.dispatch_action(action)
    }

    fn dispatch_action(&mut self, action: Action) -> Result<bool> {
        match action {
            Action::Quit => self.handle_quit(),
            Action::Save => self.handle_save(),
            Action::ExecuteCommand => self.handle_execute(),
            Action::Undo => self.handle_undo(),
            Action::Redo => self.handle_redo(),
            Action::NextMatch => self.handle_next_match(),
            Action::PrevMatch => self.handle_prev_match(),
            Action::ConfirmSearch => self.handle_confirm_search(),
            Action::JumpOlder => self.handle_jump(true),
            Action::JumpNewer => self.handle_jump(false),
            Action::EditAlternate => self.handle_alternate(),
            Action::Noop => self.handle_noop(),
            _ => self.handle_engine_action(action),
        }
    }

    fn handle_save(&mut self) -> Result<bool> {
        if !self.editor.has_file() {
            self.show_message("E32: No file name".to_string());
            return Ok(false);
        }
        self.save_and_report();
        Ok(false)
    }

    fn save_and_report(&mut self) -> bool {
        match self.editor.save() {
            Ok(outcome) => {
                let report = written_report(self.editor.filename(), outcome);
                self.show_message(report);
                true
            }
            Err(error) => self.report_error(file_error_message(error)),
        }
    }

    fn write_as_and_report(&mut self, name: &str, force: bool) -> bool {
        match self.editor.write_as(name, force) {
            Ok(outcome) => {
                let report = written_report(name, outcome);
                self.show_message(report);
                true
            }
            Err(error) => self.report_error(file_error_message(error)),
        }
    }

    fn report_error(&mut self, text: String) -> bool {
        self.show_message(text);
        false
    }

    fn handle_undo(&mut self) -> Result<bool> {
        if !self.editor.try_undo() {
            self.show_message("Already at oldest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_redo(&mut self) -> Result<bool> {
        if !self.editor.try_redo() {
            self.show_message("Already at newest change".to_string());
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_next_match(&mut self) -> Result<bool> {
        self.report_search_step(true);
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_prev_match(&mut self) -> Result<bool> {
        self.report_search_step(false);
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_confirm_search(&mut self) -> Result<bool> {
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
        Ok(false)
    }

    fn handle_engine_action(&mut self, action: Action) -> Result<bool> {
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
        let command = parse(self.editor.command_text());
        self.editor.apply(Action::CancelCommand);
        self.run_command(command)
    }

    fn run_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Write => self.handle_save(),
            Command::Quit => self.handle_quit(),
            Command::WriteQuit => self.handle_write_quit(),
            Command::ForceQuit | Command::ForceQuitAll => Ok(true),
            Command::QuitAll => self.handle_quit_all(),
            Command::WriteAll => self.handle_save_all(),
            Command::WriteQuitAll => self.handle_write_quit_all(),
            Command::Edit { name, force } => self.handle_edit(name, force),
            Command::WriteAs { name, force } => self.handle_write_as(name, force),
            Command::DeleteBuffer { spec, force } => self.handle_delete_buffer(spec, force),
            Command::ListBuffers => self.handle_list_buffers(),
            Command::NextBuffer => self.handle_buffer_step(true),
            Command::PreviousBuffer => self.handle_buffer_step(false),
            Command::AlternateBuffer => self.handle_alternate(),
            Command::SelectBuffer(arg) => self.handle_select_buffer(&arg),
            Command::SetKeymap(name) => self.switch_keymap(&name),
            Command::SetTheme(name) => self.switch_theme(&name),
            Command::SetShiftWidth(width) => self.switch_shiftwidth(width),
            Command::SetExpandTab(expandtab) => self.switch_expandtab(expandtab),
            Command::SetNumber(number) => self.switch_number(number),
            Command::SetRelativeNumber(relativenumber) => {
                self.switch_relativenumber(relativenumber)
            }
            Command::SetNumberWidth(width) => self.switch_numberwidth(width),
            Command::SetError(message) => {
                self.show_message(message);
                Ok(false)
            }
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

    fn switch_number(&mut self, number: bool) -> Result<bool> {
        self.editor.set_number(number);
        Ok(false)
    }

    fn switch_relativenumber(&mut self, relativenumber: bool) -> Result<bool> {
        self.editor.set_relativenumber(relativenumber);
        Ok(false)
    }

    fn switch_numberwidth(&mut self, width: usize) -> Result<bool> {
        self.editor.set_numberwidth(width);
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
        if !self.editor.has_file() {
            self.show_message("E32: No file name".to_string());
            return Ok(false);
        }
        if self.save_and_report() {
            return self.handle_quit();
        }
        Ok(false)
    }

    fn handle_quit(&mut self) -> Result<bool> {
        if !self.editor.is_dirty() {
            return Ok(true);
        }
        self.show_message("E37: No write since last change".to_string());
        Ok(false)
    }

    fn handle_quit_all(&mut self) -> Result<bool> {
        if !self.editor.has_unsaved() {
            return Ok(true);
        }
        self.show_message("E37: No write since last change".to_string());
        Ok(false)
    }

    fn handle_save_all(&mut self) -> Result<bool> {
        match self.editor.write_all() {
            Ok(()) => Ok(false),
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_write_quit_all(&mut self) -> Result<bool> {
        match self.editor.write_all() {
            Ok(()) => Ok(true),
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_edit(&mut self, name: Option<String>, force: bool) -> Result<bool> {
        if name.as_deref() == Some("#") {
            return self.handle_alternate();
        }
        match self.editor.edit_file(name.as_deref(), force) {
            Ok(()) => {
                self.reveal_cursor();
                Ok(false)
            }
            Err(error) => {
                self.show_message(file_error_message(error));
                Ok(false)
            }
        }
    }

    fn handle_list_buffers(&mut self) -> Result<bool> {
        self.show_message(self.editor.buffer_list());
        Ok(false)
    }

    fn handle_buffer_step(&mut self, forward: bool) -> Result<bool> {
        if forward {
            self.editor.buffer_next();
        } else {
            self.editor.buffer_previous();
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_alternate(&mut self) -> Result<bool> {
        self.editor.buffer_alternate();
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_select_buffer(&mut self, arg: &str) -> Result<bool> {
        let outcome = match arg.parse::<usize>() {
            Ok(number) => {
                if self.editor.buffer_select(number) {
                    Ok(())
                } else {
                    Err(kawi_editor::BufferError::Missing(number))
                }
            }
            Err(_) => self.editor.buffer_select_pattern(arg),
        };
        if let Err(error) = outcome {
            self.show_message(buffer_error_message(error));
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn handle_write_as(&mut self, name: String, force: bool) -> Result<bool> {
        self.write_as_and_report(&name, force);
        Ok(false)
    }

    fn handle_delete_buffer(&mut self, spec: Option<String>, force: bool) -> Result<bool> {
        match self.editor.buffer_delete(spec.as_deref(), force) {
            Ok(()) => self.reveal_cursor(),
            Err(error) => self.show_message(buffer_error_message(error)),
        }
        Ok(false)
    }

    fn handle_jump(&mut self, older: bool) -> Result<bool> {
        let outcome = if older {
            self.editor.jump_older()
        } else {
            self.editor.jump_newer()
        };
        if let Err(error) = outcome {
            self.show_message(file_error_message(error));
        }
        self.reveal_cursor();
        Ok(false)
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

fn written_report(name: &str, outcome: kawi_editor::WriteOutcome) -> String {
    let new = if outcome.created { " [New]" } else { "" };
    format!(
        "\"{name}\"{new} {}L, {}B written",
        outcome.lines, outcome.bytes
    )
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

fn buffer_error_message(error: kawi_editor::BufferError) -> String {
    match error {
        kawi_editor::BufferError::Missing(number) => {
            format!("E86: Buffer {number} does not exist")
        }
        kawi_editor::BufferError::NotFound(spec) => {
            format!("E94: No matching buffer for {spec}")
        }
        kawi_editor::BufferError::Ambiguous(spec) => {
            format!("E93: More than one match for {spec}")
        }
        kawi_editor::BufferError::NoWrite(number) => {
            format!("E89: No write since last change for buffer {number} (add ! to override)")
        }
    }
}

fn pager_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut len = 0;
        for ch in line.chars() {
            if len == width {
                lines.push(std::mem::take(&mut current));
                len = 0;
            }
            current.push(ch);
            len += 1;
        }
        lines.push(current);
    }
    lines
}

fn is_pager_dismiss(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter | KeyCode::Esc)
        || matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn file_error_message(error: kawi_editor::EditError) -> String {
    match error {
        kawi_editor::EditError::NoName => "E32: No file name".to_string(),
        kawi_editor::EditError::NoWrite => {
            "E37: No write since last change (add ! to override)".to_string()
        }
        kawi_editor::EditError::FileExists => "E13: File exists (add ! to override)".to_string(),
        kawi_editor::EditError::CannotOpen(name) => {
            format!("E484: Can't open file {name}")
        }
        kawi_editor::EditError::Io(message) => message,
        kawi_editor::EditError::InvalidMark => "E19: Mark has invalid line number".to_string(),
    }
}

fn content_size(cols: u16, rows: u16) -> (usize, usize) {
    let width = usize::from(cols).max(1);
    let height = usize::from(rows).saturating_sub(2).max(1);
    (width, height)
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
    fn ctrl_d_scrolls_half_page() {
        let text = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = App::new(Buffer::from_string(&text), Flavor::Vim);
        app.handle_resize(80, 22).expect("resize");
        for key in [
            key(KeyCode::Char('5')),
            key(KeyCode::Char('0')),
            key(KeyCode::Char('G')),
        ] {
            app.handle_key(key).expect("goto");
        }
        assert_eq!(app.cursor_position(), (49, 0));
        assert_eq!(app.viewport_top(), 40);
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
            .expect("scroll");
        assert_eq!(app.cursor_position(), (59, 0));
        assert_eq!(app.viewport_top(), 50);
    }

    #[test]
    fn zz_centers_cursor() {
        let text = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut app = App::new(Buffer::from_string(&text), Flavor::Vim);
        app.handle_resize(80, 22).expect("resize");
        for key in [
            key(KeyCode::Char('5')),
            key(KeyCode::Char('0')),
            key(KeyCode::Char('G')),
        ] {
            app.handle_key(key).expect("goto");
        }
        app.handle_key(key(KeyCode::Char('z'))).expect("arm");
        app.handle_key(key(KeyCode::Char('z'))).expect("center");
        assert_eq!(app.cursor_position(), (49, 0));
        assert_eq!(app.viewport_top(), 40);
        app.handle_key(key(KeyCode::Char('H'))).expect("top");
        assert_eq!(app.cursor_position(), (40, 0));
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
    fn vim_replace_tab_keeps_cursor_on_tab() {
        let mut app = App::new(Buffer::from_string("Hello world"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('r'))).expect("replace");
        app.handle_key(key(KeyCode::Tab)).expect("tab");
        let status = app.status_snapshot();
        assert_eq!(status.col, 0);
        assert_eq!(status.virtcol, 7);
        assert!(!status.line_empty);
    }

    #[test]
    fn status_reports_empty_line_position() {
        let mut app = App::new(Buffer::from_string("ab\n\ncd"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("down");
        let status = app.status_snapshot();
        assert!(status.line_empty);
        assert_eq!(status.row, 1);
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
    fn vim_normal_ge_moves() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        for _ in 0..7 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('e'))).expect("motion");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 6));
    }

    #[test]
    fn vim_normal_dge_deletes() {
        let mut app = App::new(Buffer::from_string("foo BAR baz"), Flavor::Vim);
        for _ in 0..7 {
            app.handle_key(key(KeyCode::Char('l'))).expect("move");
        }
        app.handle_key(key(KeyCode::Char('d'))).expect("delete");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('e'))).expect("motion");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 6));
    }

    #[test]
    fn vim_normal_tilde_toggles_and_advances() {
        let mut app = App::new(Buffer::from_string("aBc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('~'))).expect("toggle");
        assert_eq!(app.editor_mode(), Mode::Normal);
        assert_eq!(app.cursor_position(), (0, 2));
    }

    #[test]
    fn vim_normal_goto_column_start_inserts() {
        let mut app = App::new(Buffer::from_string("abc"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('l'))).expect("move");
        app.handle_key(key(KeyCode::Char('g'))).expect("g");
        app.handle_key(key(KeyCode::Char('I'))).expect("insert");
        assert_eq!(app.editor_mode(), Mode::Insert);
        assert_eq!(app.cursor_position(), (0, 0));
        app.handle_key(key(KeyCode::Char('X'))).expect("type");
        app.handle_key(key(KeyCode::Esc)).expect("escape");
        assert_eq!(app.cursor_position(), (0, 0));
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
        let _ = std::fs::remove_file(&path);
        let mut buffer = Buffer::from_string("aa\nbb");
        buffer.path = Some(path.to_string_lossy().to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.handle_save().expect("save");
        let expected = format!("\"{}\" [New] 2L, 6B written", path.to_string_lossy());
        assert_eq!(app.last_message(), Some(expected.as_str()));
        app.handle_save().expect("save again");
        let expected = format!("\"{}\" 2L, 6B written", path.to_string_lossy());
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
    fn written_report_formats_like_editor() {
        let outcome = kawi_editor::WriteOutcome {
            lines: 3,
            bytes: 12,
            created: false,
        };
        assert_eq!(
            written_report("notes.txt", outcome),
            "\"notes.txt\" 3L, 12B written"
        );
        let created = kawi_editor::WriteOutcome {
            lines: 0,
            bytes: 0,
            created: true,
        };
        assert_eq!(
            written_report("fresh.txt", created),
            "\"fresh.txt\" [New] 0L, 0B written"
        );
    }

    #[test]
    fn search_refreshes_wanted_column_for_vertical_moves() {
        let text = "\tello world\n01234567890123456789\nxabc tail";
        let mut app = App::new(kawi_buffer::Buffer::from_string(text), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('/'))).expect("search");
        for ch in "abc".chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("confirm");
        assert_eq!(app.cursor_position(), (2, 1));
        app.handle_key(key(KeyCode::Char('k'))).expect("up");
        assert_eq!(app.cursor_position(), (1, 1));
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

    fn type_command(app: &mut App, text: &str) {
        for ch in format!(":{text}").chars() {
            app.handle_key(key(KeyCode::Char(ch))).expect("type");
        }
        app.handle_key(key(KeyCode::Enter)).expect("run");
    }

    fn temp_file(name: &str, text: &str) -> String {
        let path = temp_path(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("kawi_app_{name}"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn write_as_command_names_unnamed_buffer() {
        let target = temp_path("write_as_named.txt");
        let _ = std::fs::remove_file(&target);
        let mut app = App::new(kawi_buffer::Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, &format!("w {target}"));
        assert_eq!(app.editor.filename(), target);
        assert_eq!(
            app.last_message(),
            Some(format!("\"{target}\" [New] 1L, 4B written").as_str())
        );
        assert!(!app.editor.is_dirty());
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn write_as_existing_target_reports_e13() {
        let alpha = temp_file("write_as_alpha.txt", "alpha\n");
        let exists = temp_file("write_as_exists.txt", "old\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("w {exists}"));
        assert_eq!(
            app.last_message(),
            Some("E13: File exists (add ! to override)")
        );
        type_command(&mut app, &format!("w! {exists}"));
        assert_eq!(std::fs::read_to_string(&exists).unwrap(), "alpha\n");
    }

    #[test]
    fn delete_buffer_command_works() {
        let alpha = temp_file("delete_alpha.txt", "alpha\n");
        let bravo = temp_file("delete_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, &format!("bd {alpha}"));
        assert_eq!(app.editor.listed_buffer_count(), 1);
        type_command(&mut app, "bd");
        assert_eq!(app.editor.listed_buffer_count(), 1);
        assert_eq!(app.editor.filename(), "[No Name]");
    }

    #[test]
    fn delete_modified_buffer_reports_e89() {
        let alpha = temp_file("delete_mod.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, "bd");
        assert_eq!(
            app.last_message(),
            Some("E89: No write since last change for buffer 1 (add ! to override)")
        );
    }

    #[test]
    fn select_buffer_by_partial_name() {
        let alpha = temp_file("select_name_alpha.txt", "alpha\n");
        let bravo = temp_file("select_name_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "b name_alpha");
        assert_eq!(app.editor.filename(), alpha);
        type_command(&mut app, "b zzz");
        assert_eq!(app.last_message(), Some("E94: No matching buffer for zzz"));
    }

    #[test]
    fn list_buffers_pages_when_rows_overflow() {
        let alpha = temp_file("pager_alpha_file.txt", "one\ntwo\n");
        let bravo = temp_file("pager_bravo_file.txt", "three\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_resize(40, 10).expect("resize");
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "ls");
        assert!(app.pager_lines().is_some());
        app.handle_key(key(KeyCode::Enter)).expect("dismiss");
        assert!(app.pager_lines().is_none());
    }

    #[test]
    fn pager_replays_non_dismiss_keys() {
        let alpha = temp_file("pager_replay.txt", "one\ntwo\nthree\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_resize(40, 10).expect("resize");
        type_command(&mut app, "ls");
        assert!(app.pager_lines().is_some());
        app.handle_key(key(KeyCode::Char('j'))).expect("replay");
        assert!(app.pager_lines().is_none());
        assert_eq!(app.cursor_position(), (1, 0));
    }

    #[test]
    fn edit_command_switches_files() {
        let alpha = temp_file("edit_alpha.txt", "alpha\n");
        let bravo = temp_file("edit_bravo.txt", "bravo\ncharlie\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(app.editor.filename(), bravo);
        assert_eq!(app.editor.line_count(), 2);
        type_command(&mut app, "e #");
        assert_eq!(app.editor.filename(), alpha);
    }

    #[test]
    fn edit_reload_reports_e37_when_dirty() {
        let alpha = temp_file("edit_dirty.txt", "alpha\n");
        let bravo = temp_file("edit_dirty_other.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(app.editor.filename(), bravo);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        assert_eq!(
            app.last_message(),
            Some("E37: No write since last change (add ! to override)")
        );
        assert_eq!(app.editor.filename(), bravo);
        type_command(&mut app, "e!");
        assert_eq!(app.editor.filename(), bravo);
        assert!(!app.editor.is_dirty());
    }

    #[test]
    fn quit_dirty_buffer_reports_e37() {
        let alpha = temp_file("quit_dirty.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, "q");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
        type_command(&mut app, "q!");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
    }

    #[test]
    fn quit_all_reports_hidden_changes() {
        let alpha = temp_file("quit_all.txt", "alpha\n");
        let bravo = temp_file("quit_all_other.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('x'))).expect("delete");
        type_command(&mut app, &format!("e {bravo}"));
        type_command(&mut app, "qa");
        assert_eq!(app.last_message(), Some("E37: No write since last change"));
    }

    #[test]
    fn write_unnamed_buffer_reports_e32() {
        let mut app = App::new(kawi_buffer::Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, "w");
        assert_eq!(app.last_message(), Some("E32: No file name"));
    }

    #[test]
    fn list_buffers_shows_flags() {
        let alpha = temp_file("list_buffers.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, "ls");
        let message = app.last_message().unwrap();
        assert!(message.contains("  1 %a"));
        assert!(message.contains(&format!("\"{alpha}\"")));
        assert!(message.contains("line 1"));
    }

    #[test]
    fn alternate_key_switches_buffers() {
        let alpha = temp_file("alt_alpha.txt", "alpha\n");
        let bravo = temp_file("alt_bravo.txt", "bravo\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, &format!("e {bravo}"));
        app.handle_key(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::CONTROL))
            .expect("alternate");
        assert_eq!(app.editor.filename(), alpha);
    }

    #[test]
    fn select_buffer_missing_reports_e86() {
        let alpha = temp_file("select_missing.txt", "alpha\n");
        let mut app = App::new(kawi_buffer::Buffer::from_file(&alpha).unwrap(), Flavor::Vim);
        type_command(&mut app, "b 9");
        assert_eq!(app.last_message(), Some("E86: Buffer 9 does not exist"));
        type_command(&mut app, "b nope");
        assert_eq!(app.last_message(), Some("E94: No matching buffer for nope"));
    }

    #[test]
    fn set_number_command_shows_gutter() {
        let mut app = App::new(Buffer::from_string("abc\ndef\nghi"), Flavor::Vim);
        type_command(&mut app, "set number");
        assert_eq!(app.editor.gutter_width(), 4);
        assert_eq!(
            app.editor.gutter_lines(0, 3),
            vec!["  1 ".to_string(), "  2 ".to_string(), "  3 ".to_string()]
        );
        assert_eq!(app.editor.cursor_wcol(), 4);
    }

    #[test]
    fn set_relativenumber_command_marks_cursor_line() {
        let mut app = App::new(Buffer::from_string("abc\ndef\nghi"), Flavor::Vim);
        app.handle_key(key(KeyCode::Char('j'))).expect("down");
        type_command(&mut app, "set relativenumber");
        assert_eq!(
            app.editor.gutter_lines(0, 3),
            vec!["  1 ".to_string(), "  0 ".to_string(), "  1 ".to_string()]
        );
    }

    #[test]
    fn set_numberwidth_command_validates() {
        let mut app = App::new(Buffer::from_string("abc"), Flavor::Vim);
        type_command(&mut app, "set numberwidth=0");
        assert_eq!(
            app.last_message(),
            Some("E487: Argument must be positive: numberwidth=0")
        );
        assert_eq!(app.editor.numberwidth(), 4);
        type_command(&mut app, "set numberwidth=21");
        assert_eq!(
            app.last_message(),
            Some("E474: Invalid argument: numberwidth=21")
        );
        type_command(&mut app, "set numberwidth=2");
        assert_eq!(app.editor.numberwidth(), 2);
    }

    #[test]
    fn click_maps_through_gutter_and_tab() {
        let mut app = App::new(Buffer::from_string("\tello world"), Flavor::Vim);
        type_command(&mut app, "set number");
        app.handle_mouse(press(3, 0)).expect("click gutter");
        assert_eq!(app.cursor_position(), (0, 0));
        app.handle_mouse(press(12, 0)).expect("click past tab");
        assert_eq!(app.cursor_position(), (0, 1));
        app.handle_mouse(press(4, 0)).expect("click tab start");
        assert_eq!(app.cursor_position(), (0, 0));
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
