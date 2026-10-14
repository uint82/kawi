use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyEvent, MouseEvent};
use kawi_buffer::Buffer;
use std::io::{stdout, Stdout};

use crate::keymap::Keymap;
use crate::mouse::press_position;
use crate::vim::VimKeymap;
use kawi_editor::{parse, Action, Command, Editor};
use kawi_tui::{frame::Frame, render, statusbar::StatusInfo, terminal, viewport::Viewport};

pub struct App {
    editor: Editor,
    viewport: Viewport,
    keymap: VimKeymap,
    quit_armed: bool,
}

impl App {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            editor: Editor::new(buffer),
            viewport: Viewport::new(fallback_height(), fallback_width()),
            keymap: VimKeymap::new(),
            quit_armed: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.sync_viewport_size();
        self.run_loop()
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
            mode: self.editor.mode().label(),
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
        match press_position(&mouse, &self.viewport) {
            Some((row, col)) => self.jump_to(row, col),
            None => self.handle_noop(),
        }
    }

    fn jump_to(&mut self, row: usize, col: usize) -> Result<bool> {
        self.disarm_quit();
        self.editor.apply(Action::JumpTo(row, col));
        self.reveal_cursor();
        Ok(false)
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
        self.reveal_cursor();
        Ok(false)
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
            Command::Unknown => Ok(false),
        }
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
