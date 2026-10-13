use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use std::io::{stdout, Stdout};

use crate::cursor::Cursor;
use kawi_tui::{render, terminal, viewport::Viewport};

pub struct App {
    buffer: Buffer,
    cursor: Cursor,
    viewport: Viewport,
}

impl App {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            viewport: Viewport::new(fallback_height(), fallback_width()),
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
        render::draw(
            out,
            self.buffer.as_slice(),
            self.cursor.col(),
            self.cursor.row(),
            &self.viewport,
        )
    }

    fn update(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(cols, rows) => self.handle_resize(cols, rows),
            _ => Ok(false),
        }
    }

    fn handle_resize(&mut self, cols: u16, rows: u16) -> Result<bool> {
        self.set_viewport_size(cols, rows);
        self.reveal_cursor();
        Ok(false)
    }

    fn set_viewport_size(&mut self, cols: u16, rows: u16) {
        self.viewport.set_size(usize::from(cols), usize::from(rows));
    }

    fn sync_viewport_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.set_viewport_size(cols, rows);
        }
    }

    fn reveal_cursor(&mut self) {
        self.viewport.ensure_visible(self.cursor.row());
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if is_quit(key) {
            return Ok(true);
        }
        if self.try_save(key)? {
            return Ok(false);
        }
        if self.try_apply_edit(key.code) {
            self.normalize_cursor();
            self.reveal_cursor();
            return Ok(false);
        }
        self.apply_movement(key.code);
        self.normalize_cursor();
        self.reveal_cursor();
        Ok(false)
    }

    fn try_save(&mut self, key: KeyEvent) -> Result<bool> {
        if !is_save(key) {
            return Ok(false);
        }
        self.save_buffer()?;
        Ok(true)
    }

    fn save_buffer(&mut self) -> Result<()> {
        self.buffer.save()
    }

    fn try_apply_edit(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(c) => self.insert_char_and_advance(c),
            KeyCode::Enter => self.insert_newline_and_reposition(),
            KeyCode::Backspace => self.delete_backward_and_reposition(),
            _ => false,
        }
    }

    fn insert_char_and_advance(&mut self, c: char) -> bool {
        self.insert_char_at_cursor(c);
        self.cursor.advance_col();
        true
    }

    fn insert_char_at_cursor(&mut self, c: char) {
        self.buffer
            .insert_char(self.cursor.row(), self.cursor.col(), c);
    }

    fn insert_newline_and_reposition(&mut self) -> bool {
        self.buffer
            .insert_newline(self.cursor.row(), self.cursor.col());
        self.cursor
            .move_to_line_start(self.cursor.row().saturating_add(1));
        true
    }

    fn delete_backward_and_reposition(&mut self) -> bool {
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
        true
    }

    fn apply_movement(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.cursor.move_up(),
            KeyCode::Down => self.cursor.move_down(),
            KeyCode::Left => self.cursor.move_left(),
            KeyCode::Right => self.cursor.move_right(),
            _ => {}
        }
    }

    fn normalize_cursor(&mut self) {
        self.clamp_row();
        self.clamp_col();
    }

    fn clamp_row(&mut self) {
        let max_row = self.buffer.len().saturating_sub(1);
        self.cursor.set_row(self.cursor.row().min(max_row));
    }

    fn clamp_col(&mut self) {
        let width = self.buffer.line(self.cursor.row()).map_or(0, char_count);
        self.cursor.set_col(self.cursor.col().min(width));
    }
}

fn read_next_event() -> Result<Event> {
    read().context("read terminal event")
}

fn is_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc)
        || (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('c')))
}

fn is_save(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s'))
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

fn fallback_width() -> usize {
    80
}

fn fallback_height() -> usize {
    24
}
