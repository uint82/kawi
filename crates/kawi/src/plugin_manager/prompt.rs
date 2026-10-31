use super::plan::{capability_lines, PreparedPlugin, PreparedUpdate};
use super::short_commit;

const CONFIRM_HINT: &str = "Enter to confirm, Esc to cancel";

pub(crate) enum PromptAction {
    Install(PreparedPlugin),
    Update(PreparedUpdate),
    Remove(String),
}

pub(crate) struct PluginPrompt {
    pub(crate) action: PromptAction,
    lines: Vec<String>,
    message: String,
}

impl PluginPrompt {
    pub(crate) fn install(prepared: PreparedPlugin) -> Self {
        let message = format!(
            "Install {} from {} at {} (API {})?",
            prepared.name,
            prepared.spec.source(),
            short_commit(&prepared.commit),
            prepared.api
        );
        let mut lines = vec![
            format!("plugin: {}", prepared.name),
            format!("version: {}", prepared.version),
            format!("author: {}", prepared.author),
            format!("repository: {}", prepared.spec.source()),
            format!("release: {}", prepared.tag),
            format!("commit: {}", prepared.commit),
            format!("api: {}", prepared.api),
            format!("sha256: {}", prepared.sha256),
            "host apis:".to_string(),
        ];
        lines.extend(capability_lines(&prepared.requests));
        lines.push(CONFIRM_HINT.to_string());
        Self {
            action: PromptAction::Install(prepared),
            lines,
            message,
        }
    }

    pub(crate) fn update(update: PreparedUpdate) -> Self {
        let prepared = &update.plugin;
        let changes = update.changes();
        let message = format!(
            "Update {} to {} (API {}) with {} new host API request{}?",
            prepared.name,
            short_commit(&prepared.commit),
            prepared.api,
            changes.len(),
            plural(changes.len())
        );
        let mut lines = vec![
            format!("plugin: {}", prepared.name),
            format!("version: {}", prepared.version),
            format!("commit: {}", prepared.commit),
            format!("api: {}", prepared.api),
            "new or widened host apis:".to_string(),
        ];
        lines.extend(
            changes
                .iter()
                .map(|capability| capability.name().to_string()),
        );
        lines.push(CONFIRM_HINT.to_string());
        Self {
            action: PromptAction::Update(update),
            lines,
            message,
        }
    }

    pub(crate) fn remove(name: &str) -> Self {
        Self {
            action: PromptAction::Remove(name.to_string()),
            lines: vec![
                format!("plugin: {name}"),
                "This removes the installed plugin and its lockfile entry.".to_string(),
                CONFIRM_HINT.to_string(),
            ],
            message: format!("Remove plugin {name}?"),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
