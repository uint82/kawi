use anyhow::{Context, Result};
use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use std::io::stdout;

pub struct TerminalGuard;

pub fn init() -> Result<TerminalGuard> {
    enable_raw()?;
    enter_screen()?;
    apply_initial_style()?;
    enable_mouse()?;
    Ok(TerminalGuard)
}

pub fn size() -> Result<(u16, u16)> {
    crossterm::terminal::size().context("query terminal size")
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableMouseCapture);
        let _ = execute!(stdout(), SetCursorStyle::DefaultUserShape);
        let _ = execute!(stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn enable_raw() -> Result<()> {
    enable_raw_mode().context("enable raw mode")?;
    Ok(())
}

fn enter_screen() -> Result<()> {
    execute!(stdout(), EnterAlternateScreen).context("enter alt screen")?;
    Ok(())
}

fn apply_initial_style() -> Result<()> {
    execute!(stdout(), SetCursorStyle::SteadyBlock).context("set initial cursor")?;
    Ok(())
}

fn enable_mouse() -> Result<()> {
    execute!(stdout(), EnableMouseCapture).context("enable mouse capture")?;
    Ok(())
}
