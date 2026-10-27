use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_process::find_executable_in;
use kawi_task::{TaskLoad, TaskSpec, Tasks, PROJECT_TASKS};
use std::path::{Path, PathBuf};

struct Sources {
    global: TaskLoad,
    project_path: PathBuf,
    project: Option<TaskLoad>,
}

pub(crate) fn check(paths: &StatePaths, workspace: &Path) -> Check {
    let sources = load_sources(paths, workspace);
    let mut details = Vec::new();
    let mut level = Level::Ok;
    add_source(&mut details, "tasks", &paths.tasks, &mut level);
    if sources.project.is_some() {
        add_source(
            &mut details,
            "project tasks",
            &sources.project_path,
            &mut level,
        );
    }
    add_warnings(&sources, paths, &mut details, &mut level);
    let tasks = merge_tasks(sources);
    add_task_checks(&tasks, workspace, &mut details, &mut level);
    let check = Check::new("tasks", "Tasks", level, details);
    if level >= Level::Warn {
        check.with_hint("Install missing commands or fix the task files listed above.")
    } else {
        check
    }
}

fn load_sources(paths: &StatePaths, workspace: &Path) -> Sources {
    let project_path = workspace.join(PROJECT_TASKS);
    let project = (project_path != paths.tasks).then(|| kawi_task::load(&project_path));
    Sources {
        global: kawi_task::load(&paths.tasks),
        project_path,
        project,
    }
}

fn add_source(details: &mut Vec<String>, label: &str, path: &Path, level: &mut Level) {
    if path.is_file() {
        details.push(format!("{label}: {}", path.display()));
        return;
    }
    details.push(format!("{label}: {} (missing)", path.display()));
    *level = (*level).max(Level::Info);
}

fn add_warnings(
    sources: &Sources,
    paths: &StatePaths,
    details: &mut Vec<String>,
    level: &mut Level,
) {
    for warning in sources
        .global
        .warnings
        .iter()
        .map(|warning| warning.format(&paths.tasks))
        .chain(sources.project.iter().flat_map(|project| {
            project
                .warnings
                .iter()
                .map(|warning| warning.format(&sources.project_path))
        }))
    {
        details.push(warning);
        *level = (*level).max(Level::Warn);
    }
}

fn merge_tasks(sources: Sources) -> Tasks {
    let mut tasks = sources.global.tasks;
    if let Some(project) = sources.project {
        tasks.merge(project.tasks);
    }
    tasks
}

fn add_task_checks(tasks: &Tasks, workspace: &Path, details: &mut Vec<String>, level: &mut Level) {
    let count = tasks.names().count();
    if count == 0 {
        details.push("no tasks configured".to_string());
        *level = (*level).max(Level::Info);
        return;
    }
    details.push(format!("configured: {count}"));
    for name in tasks.names() {
        let Some(task) = tasks.get(name) else {
            continue;
        };
        let (detail, task_level) = check_task(task, workspace);
        details.push(detail);
        *level = (*level).max(task_level);
    }
}

fn check_task(task: &TaskSpec, workspace: &Path) -> (String, Level) {
    let directory = task.directory(workspace);
    if !directory.is_dir() {
        return missing_directory(task, &directory);
    }
    command_detail(task, &directory)
}

fn missing_directory(task: &TaskSpec, directory: &Path) -> (String, Level) {
    (
        format!(
            "{}: working directory missing: {}",
            task.name,
            directory.display()
        ),
        Level::Warn,
    )
}

fn command_detail(task: &TaskSpec, directory: &Path) -> (String, Level) {
    match find_executable_in(&task.command, directory) {
        Some(path) => (
            format!(
                "{}: {} ({})",
                task.name,
                path.display(),
                directory.display()
            ),
            Level::Ok,
        ),
        None => (
            format!(
                "{}: command not found: {} ({})",
                task.name,
                task.command,
                directory.display()
            ),
            Level::Warn,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{executable, paths, TestDir};
    use super::*;
    use std::fs;

    fn write_project_tasks(workspace: &Path) {
        let path = workspace.join(PROJECT_TASKS);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    #[test]
    fn reports_tasks_with_resolved_commands() {
        let dir = TestDir::new("tasks_ok");
        let paths = paths(&dir.path);
        let workspace = dir.path.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        write_project_tasks(&workspace);
        let command = dir.path.join("task-bin");
        executable(&command);
        fs::write(
            &paths.tasks,
            format!("[task.build]\ncommand = {:?}\n", command.to_str().unwrap()),
        )
        .unwrap();
        let check = check(&paths, &workspace);
        assert_eq!(check.level, Level::Ok);
        assert!(check.details.iter().any(|detail| detail.contains("build")));
    }

    #[test]
    fn warns_when_a_task_command_is_missing() {
        let dir = TestDir::new("tasks_missing");
        let paths = paths(&dir.path);
        let workspace = dir.path.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        write_project_tasks(&workspace);
        fs::write(
            &paths.tasks,
            "[task.build]\ncommand = \"definitely-missing-task\"\n",
        )
        .unwrap();
        let check = check(&paths, &workspace);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }
}
