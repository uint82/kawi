mod app;
mod config;
mod event;
mod helix;
mod ipc;
mod keymap;
mod mouse;
mod options;
mod persist;
mod recovery;
mod session;
mod state;
#[cfg(test)]
mod test_support;
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| is_wait_flag(arg)) {
        return run_wait_editor(&args);
    }
    let (path, cli_flavor) = parse_cli(args);
    let restore_workspace = path.is_none();
    let cwd = std::env::current_dir()?;
    let paths = StatePaths::resolve();
    if let Some(paths) = paths.as_ref() {
        kawi_syntax::set_user_query_dir(paths.queries.clone());
    }
    let (session, warning) = load_startup_session(restore_workspace, paths.as_ref(), &cwd);
    let buffer = load_buffer(path)?;
    let mut app = App::new(buffer, cli_flavor.unwrap_or(Flavor::Vim));
    app.load_state(cli_flavor);
    apply_startup_session(&mut app, session.as_ref(), warning);
    apply_startup_recovery(&mut app, paths.as_ref(), &cwd, !restore_workspace);
    let outcome = run_app(&mut app);
    save_workspace_session(&mut app, restore_workspace, paths.as_ref(), &cwd);
    app.save_state();
    outcome
}

fn is_wait_flag(arg: &str) -> bool {
    arg == "--git-editor" || arg == "--wait"
}

fn run_wait_editor(args: &[String]) -> Result<()> {
    let Some(index) = args.iter().position(|arg| is_wait_flag(arg)) else {
        anyhow::bail!("missing editor path");
    };
    let Some(path) = args.get(index + 1) else {
        anyhow::bail!("missing editor path");
    };
    let file = std::path::absolute(Path::new(path))?;
    match ipc::wait(&file) {
        Ok(ipc::WaitOutcome::Saved) => Ok(()),
        Ok(ipc::WaitOutcome::Aborted) => std::process::exit(1),
        Err(error) => {
            eprintln!("kawi: git editor: {error}");
            std::process::exit(1);
        }
    }
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

fn apply_startup_recovery(
    app: &mut App,
    paths: Option<&StatePaths>,
    cwd: &Path,
    restrict_to_open: bool,
) {
    let Some(paths) = paths else {
        return;
    };
    if let Some(warning) = app.start_recovery(&paths.recovery, cwd, restrict_to_open) {
        app.show_startup_warning(warning);
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

fn parse_cli(args: Vec<String>) -> (Option<String>, Option<Flavor>) {
    let flavor = select_flavor(&args);
    let path = select_path(args);
    (path, flavor)
}

fn select_flavor(args: &[String]) -> Option<Flavor> {
    args.iter()
        .any(|arg| arg == "--helix")
        .then_some(Flavor::Helix)
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
        assert_eq!(parse_cli(vec![]), (None, None));
    }

    #[test]
    fn helix_flag_selects_helix() {
        assert_eq!(
            parse_cli(vec!["--helix".to_string()]),
            (None, Some(Flavor::Helix))
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
            config_log: dir.join("config.log"),
            languages: dir.join("languages.toml"),
            tasks: dir.join("tasks.toml"),
            themes: dir.join("themes"),
            queries: dir.join("queries"),
            history: dir.join("history"),
            sessions: dir.join("sessions"),
            recovery: dir.join("recovery"),
            undo: dir.join("undo"),
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
            (Some("notes.txt".to_string()), Some(Flavor::Helix))
        );
    }
}
