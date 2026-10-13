use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::Buffer;
use std::io::{stdout, Stdout};

use kawi_tui::render;

pub struct App {
    buffer: Buffer,
    row: usize,
    col: usize,
}

impl App {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            row: 0,
            col: 0,
        }
    }

    pub fn run(&mut self) -> Result<()> {
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
        render::draw(out, self.buffer.as_slice(), self.col, self.row)
    }

    fn update(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) => self.handle_key(key),
            _ => Ok(false),
        }
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
            return Ok(false);
        }
        self.apply_movement(key.code);
        self.normalize_cursor();
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
        self.advance_col();
        true
    }

    fn insert_char_at_cursor(&mut self, c: char) {
        self.buffer.insert_char(self.row, self.col, c);
    }

    fn advance_col(&mut self) {
        self.col = self.col.saturating_add(1);
    }

    fn insert_newline_and_reposition(&mut self) -> bool {
        self.buffer.insert_newline(self.row, self.col);
        self.move_to_new_line_start();
        true
    }

    fn move_to_new_line_start(&mut self) {
        self.row = self.row.saturating_add(1);
        self.col = 0;
    }

    fn delete_backward_and_reposition(&mut self) -> bool {
        let (row, col) = self.buffer.delete_backward(self.row, self.col);
        self.row = row;
        self.col = col;
        true
    }

    fn apply_movement(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            _ => {}
        }
    }

    fn move_up(&mut self) {
        self.row = self.row.saturating_sub(1);
    }

    fn move_down(&mut self) {
        self.row = self.row.saturating_add(1);
    }

    fn move_left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.col = self.col.saturating_add(1);
    }

    fn normalize_cursor(&mut self) {
        self.clamp_row();
        self.clamp_col();
    }

    fn clamp_row(&mut self) {
        let max_row = self.buffer.len().saturating_sub(1);
        self.row = self.row.min(max_row);
    }

    fn clamp_col(&mut self) {
        let width = self.buffer.line(self.row).map_or(0, char_count);
        self.col = self.col.min(width);
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
