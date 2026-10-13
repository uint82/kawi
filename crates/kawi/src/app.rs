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
            if self.update(event) {
                break;
            }
        }
        Ok(())
    }

    fn draw_frame(&self, out: &mut Stdout) -> Result<()> {
        render::draw(out, self.buffer.as_slice(), self.col, self.row)
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if is_quit(key) {
            return true;
        }
        self.apply_movement(key.code);
        self.normalize_cursor();
        false
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
        let width = self.buffer.line(self.row).map_or(0, str::len);
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
