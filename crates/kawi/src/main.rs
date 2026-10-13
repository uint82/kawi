mod app;
mod cursor;
use anyhow::Result;
use app::App;
use kawi_buffer::Buffer;
use kawi_tui::terminal;

fn main() -> Result<()> {
    let path = parse_arg();
    let buffer = load_buffer(path)?;
    let mut app = App::new(buffer);
    run_app(&mut app)
}

fn parse_arg() -> Option<String> {
    std::env::args().nth(1)
}

fn load_buffer(path: Option<String>) -> Result<Buffer> {
    match path {
        Some(p) => Buffer::from_file(p.as_str()),
        None => Ok(Buffer::from_string("")),
    }
}

fn run_app(app: &mut App) -> Result<()> {
    let _guard = terminal::init()?;
    app.run()
}
