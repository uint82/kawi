use anyhow::Result;
use kawi_lsp::{Languages, ServerConfig, ServerState};
use std::collections::BTreeMap;
use std::path::Path;

use super::lsp::one_line;
use super::App;

const TITLE: &str = "LSP status";
const WARNINGS: &str = "WARNINGS";
const DETAILS: &str = "DETAILS";
const NO_SERVERS: &str = "no language servers configured";
const NO_LANGUAGES: &str = "-";
const NO_COMMAND: &str = "-";
const COLUMNS: usize = 4;
const HEADERS: [&str; COLUMNS] = ["SERVER", "STATE", "LANGUAGES", "COMMAND"];

pub(super) struct StatusContext<'a> {
    pub(super) languages: &'a Languages,
    pub(super) states: &'a BTreeMap<String, ServerState>,
    pub(super) messages: &'a BTreeMap<String, String>,
    pub(super) warnings_path: Option<&'a Path>,
}

struct StatusRow<'a> {
    server: &'a str,
    state: &'static str,
    languages: String,
    command: String,
    path: Option<&'a Path>,
}

impl App {
    pub(super) fn show_lsp_status(&mut self) -> Result<bool> {
        let lines = self.lsp.status_report();
        self.show_help_text("lsp-status", lines)?;
        Ok(false)
    }
}

pub(super) fn report(context: &StatusContext<'_>) -> Vec<String> {
    let rows = build_rows(context.languages, context.states);
    let mut lines = vec![TITLE.to_string(), String::new()];
    let warnings = render_warnings(context.languages, context.warnings_path);
    if !warnings.is_empty() {
        lines.extend(warnings);
        lines.push(String::new());
    }
    if rows.is_empty() {
        lines.push(NO_SERVERS.to_string());
        return lines;
    }
    lines.extend(render_rows(&rows));
    let details = render_details(&rows, context.messages);
    if !details.is_empty() {
        lines.push(String::new());
        lines.extend(details);
    }
    lines
}

fn build_rows<'a>(
    languages: &'a Languages,
    states: &BTreeMap<String, ServerState>,
) -> Vec<StatusRow<'a>> {
    languages
        .servers()
        .map(|(name, config)| StatusRow {
            server: name,
            state: state_label(languages, name, states),
            languages: languages_label(languages, name),
            command: command_label(config),
            path: languages.server_path(name),
        })
        .collect()
}

fn state_label(
    languages: &Languages,
    server: &str,
    states: &BTreeMap<String, ServerState>,
) -> &'static str {
    let Some(config) = languages.server(server) else {
        return "unused";
    };
    if !config.enabled {
        return "disabled";
    }
    if !languages.server_is_used(server) {
        return "unused";
    }
    if let Some(state) = states.get(server) {
        return state_name(*state);
    }
    if config.command.trim().is_empty() {
        return "no command";
    }
    if languages.server_path(server).is_none() {
        return "missing";
    }
    "idle"
}

fn state_name(state: ServerState) -> &'static str {
    match state {
        ServerState::Starting => "starting",
        ServerState::Initializing => "initializing",
        ServerState::Ready => "ready",
        ServerState::Missing => "missing",
        ServerState::Failed => "failed",
        ServerState::Stopped => "stopped",
    }
}

fn languages_label(languages: &Languages, server: &str) -> String {
    let names: Vec<&str> = languages
        .languages_using(server)
        .map(|language| language.name.as_str())
        .collect();
    if names.is_empty() {
        return NO_LANGUAGES.to_string();
    }
    names.join(", ")
}

fn command_label(config: &ServerConfig) -> String {
    if config.command.trim().is_empty() {
        return NO_COMMAND.to_string();
    }
    std::iter::once(config.command.as_str())
        .chain(config.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_rows(rows: &[StatusRow<'_>]) -> Vec<String> {
    let widths = column_widths(rows);
    let mut lines = vec![row_line(HEADERS, &widths)];
    lines.extend(rows.iter().map(|row| {
        row_line(
            [row.server, row.state, &row.languages, &row.command],
            &widths,
        )
    }));
    lines
}

fn column_widths(rows: &[StatusRow<'_>]) -> [usize; COLUMNS] {
    let mut widths = HEADERS.map(str::len);
    for row in rows {
        widths[0] = widths[0].max(row.server.len());
        widths[1] = widths[1].max(row.state.len());
        widths[2] = widths[2].max(row.languages.len());
        widths[3] = widths[3].max(row.command.len());
    }
    widths
}

fn row_line(cells: [&str; COLUMNS], widths: &[usize; COLUMNS]) -> String {
    format!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {}",
        cells[0],
        cells[1],
        cells[2],
        cells[3],
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2]
    )
    .trim_end()
    .to_string()
}

fn render_warnings(languages: &Languages, path: Option<&Path>) -> Vec<String> {
    let warnings = languages.warnings();
    if warnings.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![WARNINGS.to_string()];
    lines.extend(warnings.iter().map(|warning| match path {
        Some(path) => warning.format(path),
        None => warning.to_string(),
    }));
    lines
}

fn render_details(rows: &[StatusRow<'_>], messages: &BTreeMap<String, String>) -> Vec<String> {
    let mut details = Vec::new();
    for row in rows {
        if let Some(message) = messages.get(row.server) {
            details.push(format!("{}: {}", row.server, one_line(message)));
        }
        if let Some(path) = row.path {
            details.push(format!("{}: {}", row.server, path.display()));
        }
    }
    if details.is_empty() {
        return Vec::new();
    }
    std::iter::once(DETAILS.to_string())
        .chain(details)
        .collect()
}

#[cfg(test)]
mod tests;
