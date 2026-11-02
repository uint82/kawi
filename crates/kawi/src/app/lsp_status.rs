use anyhow::Result;
use kawi_lsp::{Languages, ServerConfig, ServerState};
use std::collections::BTreeMap;

use super::lsp::one_line;
use super::App;

const TITLE: &str = "LSP status";
const DETAILS: &str = "DETAILS";
const NO_SERVERS: &str = "no language servers configured";
const NO_LANGUAGES: &str = "-";
const NO_COMMAND: &str = "-";
const COLUMNS: usize = 4;
const HEADERS: [&str; COLUMNS] = ["SERVER", "STATE", "LANGUAGES", "COMMAND"];

struct StatusRow {
    server: String,
    state: &'static str,
    languages: String,
    command: String,
}

impl App {
    pub(super) fn show_lsp_status(&mut self) -> Result<bool> {
        self.show_pager(self.lsp.status_report());
        Ok(false)
    }
}

pub(super) fn report(
    languages: &Languages,
    states: &BTreeMap<String, ServerState>,
    messages: &BTreeMap<String, String>,
) -> Vec<String> {
    let rows = build_rows(languages, states);
    let mut lines = vec![TITLE.to_string(), String::new()];
    if rows.is_empty() {
        lines.push(NO_SERVERS.to_string());
        return lines;
    }
    lines.extend(render_rows(&rows));
    let details = render_details(&rows, messages);
    if !details.is_empty() {
        lines.push(String::new());
        lines.extend(details);
    }
    lines
}

fn build_rows(languages: &Languages, states: &BTreeMap<String, ServerState>) -> Vec<StatusRow> {
    languages
        .servers()
        .map(|(name, config)| StatusRow {
            server: name.to_string(),
            state: state_label(languages, name, states),
            languages: languages_label(languages, name),
            command: command_label(config),
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

fn render_rows(rows: &[StatusRow]) -> Vec<String> {
    let widths = column_widths(rows);
    let mut lines = vec![row_line(HEADERS, &widths)];
    lines.extend(rows.iter().map(|row| {
        row_line(
            [&row.server, row.state, &row.languages, &row.command],
            &widths,
        )
    }));
    lines
}

fn column_widths(rows: &[StatusRow]) -> [usize; COLUMNS] {
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

fn render_details(rows: &[StatusRow], messages: &BTreeMap<String, String>) -> Vec<String> {
    let details: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let message = messages.get(&row.server)?;
            Some(format!("{}: {}", row.server, one_line(message)))
        })
        .collect();
    if details.is_empty() {
        return Vec::new();
    }
    std::iter::once(DETAILS.to_string())
        .chain(details)
        .collect()
}

#[cfg(test)]
mod tests;
