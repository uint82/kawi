mod app;
mod event;
mod helix;
mod keymap;
mod mouse;
mod state;
mod vim;
use anyhow::Result;
use app::App;
use kawi_buffer::Buffer;
use kawi_editor::paths;
use kawi_tui::terminal;
use keymap::Flavor;
use std::path::Path;

fn main() -> Result<()> {
    let (path, flavor) = parse_cli(std::env::args().skip(1).collect());
    let buffer = load_buffer(path)?;
    let mut app = App::new(buffer, flavor);
    app.load_state();
    let outcome = run_app(&mut app);
    app.save_state();
    outcome
}

fn parse_cli(args: Vec<String>) -> (Option<String>, Flavor) {
    let flavor = select_flavor(&args);
    let path = select_path(args);
    (path, flavor)
}

fn select_flavor(args: &[String]) -> Flavor {
    if args.iter().any(|arg| arg == "--helix") {
        return Flavor::Helix;
    }
    Flavor::Vim
}

fn select_path(args: Vec<String>) -> Option<String> {
    args.into_iter().find(|arg| arg != "--helix")
}

fn load_buffer(path: Option<String>) -> Result<Buffer> {
    let Some(path) = path else {
        return Ok(Buffer::from_string(""));
    };
    let cwd = std::env::current_dir()?;
    let display = if Path::new(&path).is_absolute() {
        paths::shorten(&paths::absolute(&path, &cwd), &cwd)
    } else {
        path
    };
    Buffer::open(&display)
}

fn run_app(app: &mut App) -> Result<()> {
    let _guard = terminal::init()?;
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_vim_without_file() {
        assert_eq!(parse_cli(vec![]), (None, Flavor::Vim));
    }

    #[test]
    fn helix_flag_selects_helix() {
        assert_eq!(
            parse_cli(vec!["--helix".to_string()]),
            (None, Flavor::Helix)
        );
    }

    #[test]
    fn file_and_flag_combine() {
        assert_eq!(
            parse_cli(vec!["notes.txt".to_string(), "--helix".to_string()]),
            (Some("notes.txt".to_string()), Flavor::Helix)
        );
    }
}
