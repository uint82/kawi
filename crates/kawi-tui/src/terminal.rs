use anyhow::{Context, Result};
use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableFocusChange, EnableFocusChange},
    execute,
    style::{Print, ResetColor},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use std::io::stdout;

const MOUSE_ENABLE: &str = "\x1b[?1002h\x1b[?1006h";
const MOUSE_DISABLE: &str = "\x1b[?1006l\x1b[?1002l";
const PASTE_ENABLE: &str = "\x1b[?2004h";
const PASTE_DISABLE: &str = "\x1b[?2004l";

pub struct TerminalGuard;

pub fn init() -> Result<TerminalGuard> {
    enable_raw()?;
    enter_screen()?;
    apply_initial_style()?;
    enable_mouse()?;
    enable_bracketed_paste()?;
    enable_focus_change()?;
    Ok(TerminalGuard)
}

pub fn size() -> Result<(u16, u16)> {
    crossterm::terminal::size().context("query terminal size")
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableFocusChange);
        let _ = execute!(stdout(), Print(PASTE_DISABLE));
        let _ = execute!(stdout(), Print(MOUSE_DISABLE));
        let _ = execute!(stdout(), SetCursorStyle::DefaultUserShape);
        let _ = execute!(stdout(), ResetColor);
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
    execute!(stdout(), Print(MOUSE_ENABLE)).context("enable mouse capture")?;
    Ok(())
}

fn enable_bracketed_paste() -> Result<()> {
    execute!(stdout(), Print(PASTE_ENABLE)).context("enable bracketed paste")?;
    Ok(())
}

fn enable_focus_change() -> Result<()> {
    execute!(stdout(), EnableFocusChange).context("enable focus events")?;
    Ok(())
}
