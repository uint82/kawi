use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_dap::DebugConfig;
use std::path::Path;

pub(crate) fn check(paths: &StatePaths, workspace: &Path) -> Check {
    let project = workspace.join(".kawi/debug.toml");
    let config = DebugConfig::load(&[paths.debug.clone(), project.clone()]);
    let mut details = Vec::new();
    let mut level = file_level(paths, &project, &config, &mut details);
    let mut adapters = 0;
    let mut missing = 0;
    for (name, adapter) in config.adapters() {
        adapters += 1;
        match kawi_process::find_executable(&adapter.command) {
            Some(path) => details.push(format!("{name}: {} ({})", adapter.command, path.display())),
            None => {
                missing += 1;
                details.push(format!("{name}: {} not found in PATH", adapter.command));
            }
        }
    }
    let mut configurations = 0;
    for configuration in config.configurations() {
        configurations += 1;
        if !config
            .adapters()
            .any(|(name, _)| name == configuration.adapter)
        {
            missing += 1;
            details.push(format!(
                "configuration {}: adapter {} not found",
                configuration.name, configuration.adapter
            ));
        }
    }
    if adapters == 0 {
        details.push("no debug adapters configured".to_string());
    }
    if configurations == 0 {
        details.push("no debug configurations configured".to_string());
    }
    if missing > 0 {
        level = level.max(Level::Warn);
    } else if adapters == 0 && configurations == 0 {
        level = level.max(Level::Info);
    }
    let check = Check::new("debug", "Debug", level, details);
    if missing > 0 {
        check.with_hint("Install missing debug adapters or update debug.toml.")
    } else {
        check
    }
}

fn file_level(
    paths: &StatePaths,
    project: &Path,
    config: &DebugConfig,
    details: &mut Vec<String>,
) -> Level {
    let mut level = Level::Info;
    if paths.debug.is_file() {
        details.push(format!("debug: {}", paths.debug.display()));
        level = Level::Ok;
    }
    if project.is_file() {
        details.push(format!("debug: {}", project.display()));
        level = Level::Ok;
    }
    if level == Level::Info {
        details.push(format!(
            "debug: {} (missing, using no configurations)",
            paths.debug.display()
        ));
    }
    for warning in config.warnings() {
        details.push(warning.format());
        level = level.max(Level::Warn);
    }
    level
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{executable, paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn reports_a_resolved_debug_adapter() {
        let dir = TestDir::new("debug_ok");
        let paths = paths(&dir.path);
        let adapter = dir.path.join("fake-dap");
        executable(&adapter);
        fs::write(
            &paths.debug,
            format!(
                "[adapter.fake]\ncommand = {:?}\n\n\
                 [configuration.rust]\nadapter = \"fake\"\nrequest = \"launch\"\n",
                adapter.to_str().unwrap()
            ),
        )
        .unwrap();
        let check = check(&paths, &dir.path);
        assert_eq!(check.level, Level::Ok);
        assert!(check.details.iter().any(|detail| detail.contains("fake")));
    }

    #[test]
    fn warns_when_a_debug_adapter_is_missing() {
        let dir = TestDir::new("debug_missing");
        let paths = paths(&dir.path);
        fs::write(
            &paths.debug,
            "[adapter.fake]\ncommand = \"kawi-missing-dap\"\n",
        )
        .unwrap();
        let check = check(&paths, &dir.path);
        assert_eq!(check.level, Level::Warn);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("not found")));
    }

    #[test]
    fn reports_an_empty_debug_configuration() {
        let dir = TestDir::new("debug_empty");
        let paths = paths(&dir.path);
        let check = check(&paths, &dir.path);
        assert_eq!(check.level, Level::Info);
    }
}
