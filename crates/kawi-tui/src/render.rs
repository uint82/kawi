use anyhow::{Context, Result};
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{Stdout, Write};

pub fn draw(out: &mut Stdout, lines: &[String], col: usize, row: usize) -> Result<()> {
    clear_all(out)?;
    draw_lines(out, lines)?;
    place_cursor(out, col, row)?;
    flush(out)?;
    Ok(())
}

fn clear_all(out: &mut Stdout) -> Result<()> {
    execute!(out, Clear(ClearType::All)).context("clear screen")?;
    Ok(())
}

fn draw_lines(out: &mut Stdout, lines: &[String]) -> Result<()> {
    for (idx, line) in lines.iter().enumerate() {
        draw_one_line(out, idx, line)?;
    }
    Ok(())
}

fn draw_one_line(out: &mut Stdout, row: usize, text: &str) -> Result<()> {
    execute!(out, MoveTo(0, saturate_u16(row))).context("move to line start")?;
    write!(out, "{text}").context("write line")?;
    Ok(())
}

fn place_cursor(out: &mut Stdout, col: usize, row: usize) -> Result<()> {
    execute!(out, MoveTo(saturate_u16(col), saturate_u16(row))).context("place cursor")?;
    Ok(())
}

fn flush(out: &mut Stdout) -> Result<()> {
    out.flush().context("flush stdout")?;
    Ok(())
}

fn saturate_u16(v: usize) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}
