use kawi_completion::Documentation;
use kawi_help::{HelpIndex, HelpPage};
use kawi_tui::markdown::{self, MarkdownBlock, MarkdownLine};

use super::{App, COMMANDS_TOPIC, OPTIONS_TOPIC, OPTION_TOPIC_PREFIX};
use crate::options::{ConfigSection, OptionKind, OptionScope};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HelpLink {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HelpView {
    pub(super) topic: String,
    pub(super) lines: Vec<String>,
    pub(super) links: std::collections::BTreeMap<usize, Vec<HelpLink>>,
}

impl App {
    pub(super) fn help_view(&self, topic: &str) -> Option<HelpView> {
        if topic.eq_ignore_ascii_case(OPTIONS_TOPIC) {
            return Some(self.options_view());
        }
        if topic.eq_ignore_ascii_case(COMMANDS_TOPIC) {
            return Some(commands_view());
        }
        if let Some(name) = topic.strip_prefix(OPTION_TOPIC_PREFIX) {
            return self.option_view(name.trim());
        }
        HelpIndex.resolve(topic).map(page_view)
    }

    fn options_view(&self) -> HelpView {
        let mut lines = vec![
            "Options".to_string(),
            String::new(),
            "Set options with :set name=value. Query with :set name? and reset with :set name&."
                .to_string(),
            String::new(),
        ];
        for spec in crate::options::OPTIONS {
            lines.push(spec.name.to_string());
            lines.push(format!(
                "  default: {}  current: {}  scope: {}",
                spec.default_value().display(),
                self.option_value(&spec).display(),
                scope_name(spec.scope)
            ));
            lines.push(format!(
                "  config: [{}] {}  type: {}",
                section_name(spec.section),
                spec.config_key,
                kind_name(&spec.kind)
            ));
            if !spec.aliases.is_empty() {
                lines.push(format!("  aliases: {}", spec.aliases.join(", ")));
            }
            lines.push(String::new());
        }
        plain_view(OPTIONS_TOPIC, lines)
    }

    fn option_view(&self, name: &str) -> Option<HelpView> {
        let spec = crate::options::lookup(name)?;
        let mut lines = vec![
            format!("Option {}", spec.name),
            String::new(),
            format!("default: {}", spec.default_value().display()),
            format!("current: {}", self.option_value(spec).display()),
            format!("scope: {}", scope_name(spec.scope)),
            format!("type: {}", kind_name(&spec.kind)),
            format!(
                "config: [{}] {}",
                section_name(spec.section),
                spec.config_key
            ),
        ];
        if !spec.aliases.is_empty() {
            lines.push(format!("aliases: {}", spec.aliases.join(", ")));
        }
        lines.push(String::new());
        lines.push(format!("Set with :set {}=<value>", spec.name));
        Some(plain_view(&format!("option {}", spec.name), lines))
    }
}
fn page_view(page: HelpPage) -> HelpView {
    let document = markdown::parse(&Documentation::from_markup(page.body(), true));
    let mut lines = Vec::new();
    let mut links = BTreeMap::new();
    for block in document.blocks {
        match block {
            MarkdownBlock::Text(text) => push_lines(text, &mut lines, &mut links),
            MarkdownBlock::Code(code) => push_lines(code.lines, &mut lines, &mut links),
            MarkdownBlock::Rule => {
                lines.push("-".repeat(20));
            }
        }
    }
    push_related(page, &mut lines, &mut links);
    HelpView {
        topic: page.slug.to_string(),
        lines,
        links,
    }
}

fn push_related(
    page: HelpPage,
    lines: &mut Vec<String>,
    links: &mut BTreeMap<usize, Vec<HelpLink>>,
) {
    if page.related.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("See also".to_string());
    lines.push(String::new());
    for related in page.related {
        let Some(target) = HelpIndex.resolve(related) else {
            continue;
        };
        let row = lines.len();
        let text = format!("- {}", target.title);
        links.entry(row).or_default().push(HelpLink {
            start: 2,
            end: 2 + target.title.chars().count(),
            target: format!("help:{}", target.slug),
        });
        lines.push(text);
    }
}

fn push_lines(
    source: Vec<MarkdownLine>,
    lines: &mut Vec<String>,
    links: &mut BTreeMap<usize, Vec<HelpLink>>,
) {
    for line in source {
        let row = lines.len();
        let mut column = 0;
        for span in &line.spans {
            let width = span.text.chars().count();
            if let Some(target) = span.link.as_ref() {
                links.entry(row).or_default().push(HelpLink {
                    start: column,
                    end: column.saturating_add(width),
                    target: target.clone(),
                });
            }
            column = column.saturating_add(width);
        }
        lines.push(line.text());
    }
}

fn plain_view(topic: &str, lines: Vec<String>) -> HelpView {
    HelpView {
        topic: topic.to_string(),
        lines,
        links: BTreeMap::new(),
    }
}

fn commands_view() -> HelpView {
    let mut lines = vec![
        "Commands".to_string(),
        String::new(),
        "Run a command by typing : followed by its name.".to_string(),
        String::new(),
    ];
    lines.extend(
        kawi_editor::command::COMMAND_NAMES
            .iter()
            .map(|name| format!(":{name}")),
    );
    plain_view(COMMANDS_TOPIC, lines)
}

fn scope_name(scope: OptionScope) -> &'static str {
    match scope {
        OptionScope::Global => "global",
        OptionScope::Buffer => "buffer",
        OptionScope::Window => "window",
    }
}

fn section_name(section: ConfigSection) -> &'static str {
    match section {
        ConfigSection::Ui => "ui",
        ConfigSection::Editor => "editor",
        ConfigSection::History => "history",
    }
}

fn kind_name(kind: &OptionKind) -> String {
    match kind {
        OptionKind::Bool => "boolean".to_string(),
        OptionKind::Int { min, max } => format!("integer {min}..{max}"),
        OptionKind::Enum(_) => "enum".to_string(),
        OptionKind::Text => "text".to_string(),
    }
}
