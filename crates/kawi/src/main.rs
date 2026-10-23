mod app;
mod event;
mod helix;
mod keymap;
mod mouse;
mod persist;
mod recovery;
mod session;
mod state;
mod text;
mod vim;
mod workspace;
use anyhow::Result;
use app::App;
use kawi_buffer::Buffer;
use kawi_editor::paths;
use kawi_tui::terminal;
use keymap::Flavor;
use state::StatePaths;
use std::path::Path;

fn main() -> Result<()> {
    let (path, flavor) = parse_cli(std::env::args().skip(1).collect());
    let restore_workspace = path.is_none();
    let cwd = std::env::current_dir()?;
    let paths = StatePaths::resolve();
    let (session, warning) = load_startup_session(restore_workspace, paths.as_ref(), &cwd);
    let buffer = load_buffer(path)?;
    let mut app = App::new(buffer, flavor);
    app.load_state();
    apply_startup_session(&mut app, session.as_ref(), warning);
    let outcome = run_app(&mut app);
    save_workspace_session(&mut app, restore_workspace, paths.as_ref(), &cwd);
    app.save_state();
    outcome
}

fn load_startup_session(
    enabled: bool,
    paths: Option<&StatePaths>,
    cwd: &Path,
) -> (Option<session::Session>, Option<String>) {
    if !enabled {
        return (None, None);
    }
    let Some(paths) = paths else {
        return (None, None);
    };
    match session::load_for_cwd(&paths.sessions, cwd) {
        Ok(session) => (session, None),
        Err(warning) => (None, Some(warning)),
    }
}

fn apply_startup_session(
    app: &mut App,
    session: Option<&session::Session>,
    warning: Option<String>,
) {
    let mut warnings: Vec<String> = warning.into_iter().collect();
    if let Some(session) = session {
        warnings.extend(app.restore_session(session));
    }
    if !warnings.is_empty() {
        app.show_startup_warning(warnings.join("\n"));
    }
}

fn save_workspace_session(app: &mut App, enabled: bool, paths: Option<&StatePaths>, cwd: &Path) {
    if !enabled {
        return;
    }
    let Some(paths) = paths else {
        return;
    };
    let Some(session) = app.session_snapshot() else {
        return;
    };
    let _ = session::save_for_cwd(&paths.sessions, cwd, &session);
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
    fn workspace_session_round_trips_through_startup_helpers() {
        let dir = std::env::temp_dir().join(format!(
            "kawi_main_session_{}_{}",
            std::process::id(),
            "roundtrip"
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("alpha.txt");
        std::fs::write(&file, "alpha\n").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let paths = StatePaths {
            config: dir.join("config.toml"),
            languages: dir.join("languages.toml"),
            history: dir.join("history"),
            sessions: dir.join("sessions"),
        };
        let mut app = App::new(
            Buffer::from_file(file.to_str().unwrap()).unwrap(),
            Flavor::Vim,
        );
        save_workspace_session(&mut app, true, Some(&paths), &cwd);
        let (session, warning) = load_startup_session(true, Some(&paths), &cwd);
        assert_eq!(warning, None);
        let session = session.expect("session");
        assert_eq!(session.buffers.len(), 1);
        assert_eq!(session.active_window, 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_and_flag_combine() {
        assert_eq!(
            parse_cli(vec!["notes.txt".to_string(), "--helix".to_string()]),
            (Some("notes.txt".to_string()), Flavor::Helix)
        );
    }
}
