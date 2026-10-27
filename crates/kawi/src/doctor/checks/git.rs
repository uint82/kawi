use super::super::report::{Check, Level};
use kawi_git::{git_dir, git_version, repository_root};
use kawi_process::find_executable;
use std::path::Path;

pub(crate) fn check(workspace: &Path) -> Check {
    let Some(version) = git_version() else {
        return missing_git();
    };
    let mut details = vec![format!(
        "git: {}.{}.{}",
        version.major, version.minor, version.patch
    )];
    let mut level = Level::Ok;
    match repository_root(workspace) {
        Ok(Some(root)) => {
            details.push(format!("repository: {}", root.display()));
            level = level.max(add_git_dir(&root, &mut details));
        }
        Ok(None) => {
            details.push("repository: not a Git repository".to_string());
            level = level.max(Level::Info);
        }
        Err(error) => {
            details.push(format!("repository: {error}"));
            level = level.max(Level::Warn);
        }
    }
    let check = Check::new("git", "Git", level, details);
    if level >= Level::Warn {
        check.with_hint("Check the Git installation and repository.")
    } else {
        check
    }
}

fn missing_git() -> Check {
    let detail = match find_executable("git") {
        Some(path) => format!(
            "git: found {} but its version could not be read",
            path.display()
        ),
        None => "git: not found in PATH".to_string(),
    };
    Check::new("git", "Git", Level::Warn, vec![detail])
        .with_hint("Install Git to enable Git integration.")
}

fn add_git_dir(root: &Path, details: &mut Vec<String>) -> Level {
    match git_dir(root) {
        Ok(Some(path)) => {
            details.push(format!("git dir: {}", path.display()));
            Level::Ok
        }
        Ok(None) => {
            details.push("git dir: not found".to_string());
            Level::Warn
        }
        Err(error) => {
            details.push(format!("git dir: {error}"));
            Level::Warn
        }
    }
}
