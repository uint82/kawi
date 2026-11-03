mod checks;
mod report;

use crate::state::StatePaths;
use anyhow::{Context, Result};
use report::Report;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DOCTOR: &str = "doctor";
const DOCTOR_USAGE: &str = "\
Usage: kawi doctor [OPTIONS]

Options:
  --json          Print a machine-readable report
  --strict        Exit with failure when warnings are present
  --path <PATH>   Project file or directory to inspect
  -h, --help      Print this help";

pub(crate) fn run_cli(args: &[String]) -> Result<Option<i32>> {
    if args.first().map(String::as_str) != Some(DOCTOR) {
        return Ok(None);
    }
    match parse_args(&args[1..]) {
        Ok(Command::Help) => {
            println!("{DOCTOR_USAGE}");
            Ok(Some(0))
        }
        Ok(Command::Run(options)) => run(options).map(Some),
        Err(error) => {
            eprintln!("kawi doctor: {error}\n\n{DOCTOR_USAGE}");
            Ok(Some(2))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(Options),
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    path: PathBuf,
    json: bool,
    strict: bool,
}

#[derive(Default)]
struct ParsedOptions {
    path: Option<PathBuf>,
    json: bool,
    strict: bool,
}

impl ParsedOptions {
    fn set_path(&mut self, value: Option<&String>) -> std::result::Result<(), String> {
        let value = value.ok_or_else(|| "--path requires a value".to_string())?;
        if self.path.replace(PathBuf::from(value)).is_some() {
            return Err("--path may only be used once".to_string());
        }
        Ok(())
    }

    fn finish(self) -> std::result::Result<Options, String> {
        let path = match self.path {
            Some(path) => path,
            None => std::env::current_dir()
                .map_err(|error| format!("cannot determine current directory: {error}"))?,
        };
        Ok(Options {
            path,
            json: self.json,
            strict: self.strict,
        })
    }
}

fn parse_args(args: &[String]) -> std::result::Result<Command, String> {
    let mut parsed = ParsedOptions::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--json" => parsed.json = true,
            "--strict" => parsed.strict = true,
            "--path" => parsed.set_path(args.next())?,
            option => return Err(format!("unknown option {option}")),
        }
    }
    Ok(Command::Run(parsed.finish()?))
}

fn run(options: Options) -> Result<i32> {
    let target = std::fs::canonicalize(&options.path)
        .with_context(|| format!("cannot access {}", options.path.display()))?;
    let workspace = project_workspace(&target);
    let paths = StatePaths::resolve();
    let report = checks::collect(&target, &workspace, paths.as_ref());
    write_report(&report, options.json)?;
    Ok(report.exit_code(options.strict))
}

fn project_workspace(target: &Path) -> PathBuf {
    kawi_picker::project_root(working_directory(target))
}

pub(super) fn working_directory(target: &Path) -> &Path {
    if target.is_dir() {
        target
    } else {
        target.parent().unwrap_or(target)
    }
}

pub(crate) struct HealthRequest<'a> {
    pub(crate) target: &'a Path,
    pub(crate) workspace: &'a Path,
    pub(crate) paths: Option<&'a StatePaths>,
    pub(crate) filter: Option<&'a str>,
}

pub(crate) fn health_text(request: HealthRequest<'_>) -> String {
    checks::collect(request.target, request.workspace, request.paths)
        .filtered(request.filter)
        .to_text_with_title("Kawi health")
}

fn write_report(report: &Report, json: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    if json {
        serde_json::to_writer_pretty(&mut output, report).context("write doctor report")?;
        writeln!(output).context("write doctor report")?;
    } else {
        output
            .write_all(report.to_text().as_bytes())
            .context("write doctor report")?;
    }
    output.flush().context("flush doctor report")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_options() {
        let current = std::env::current_dir().unwrap();
        assert_eq!(
            parse_args(&[]),
            Ok(Command::Run(Options {
                path: current,
                json: false,
                strict: false,
            }))
        );
    }

    #[test]
    fn parses_json_strict_and_path() {
        assert_eq!(
            parse_args(&[
                "--json".to_string(),
                "--strict".to_string(),
                "--path".to_string(),
                "project".to_string(),
            ]),
            Ok(Command::Run(Options {
                path: PathBuf::from("project"),
                json: true,
                strict: true,
            }))
        );
    }

    #[test]
    fn help_short_circuits_other_options() {
        assert_eq!(
            parse_args(&["--json".to_string(), "--help".to_string()]),
            Ok(Command::Help)
        );
    }

    #[test]
    fn health_text_uses_the_health_title_and_filter() {
        let text = health_text(HealthRequest {
            target: Path::new("/project"),
            workspace: Path::new("/project"),
            paths: None,
            filter: Some("paths"),
        });
        assert!(text.starts_with("Kawi health"));
        assert!(text.contains("[error] Paths"));
    }

    #[test]
    fn health_text_filters_out_other_checks() {
        let text = health_text(HealthRequest {
            target: Path::new("/project"),
            workspace: Path::new("/project"),
            paths: None,
            filter: Some("languages"),
        });
        assert!(!text.contains("Paths"));
    }

    #[test]
    fn rejects_unknown_and_incomplete_options() {
        assert!(parse_args(&["--unknown".to_string()]).is_err());
        assert!(parse_args(&["--path".to_string()]).is_err());
        assert!(parse_args(&[
            "--path".to_string(),
            "one".to_string(),
            "--path".to_string(),
            "two".to_string(),
        ])
        .is_err());
    }
}
