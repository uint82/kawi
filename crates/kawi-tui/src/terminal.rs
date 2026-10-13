use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use std::io::stdout;

pub struct TerminalGuard;

pub fn init() -> Result<TerminalGuard> {
    enable_raw()?;
    enter_screen()?;
    Ok(TerminalGuard)
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
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
