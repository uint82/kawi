use kawi_editor::Mode;

mod commands;
mod objects;
mod sequences;

#[cfg(test)]
mod tests;

pub(super) use commands::{bracket_action, fold_action, g_action, place_action, visual_g_action};
pub(super) use objects::object_target;
pub(super) use sequences::{is_sequence_prefix, sequence_action};

use commands::{
    BRACKET_COMMANDS, CASE_G_MOTIONS, CASE_MOTIONS, COMMENT_MOTIONS, FOLD_COMMANDS, G_COMMANDS,
    OPERATOR_G_MOTIONS, PLACE_COMMANDS, VISUAL_G_COMMANDS,
};
use objects::OBJECT_TARGETS;
use sequences::OPERATOR_SEQUENCES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GuideRow {
    pub(crate) key: char,
    pub(crate) aliases: &'static [char],
    pub(crate) description: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Guide {
    pub(crate) rows: Vec<GuideRow>,
    pub(crate) hint: Option<&'static str>,
}

pub(crate) fn guide_breadcrumb(prefix: &str) -> String {
    let mut breadcrumb = String::new();
    for character in normalize_prefix(prefix).chars() {
        if !breadcrumb.is_empty() {
            breadcrumb.push(' ');
        }
        breadcrumb.push(character);
    }
    breadcrumb
}

pub(crate) fn guide(prefix: &str, mode: Mode) -> Option<Guide> {
    let prefix = normalize_prefix(prefix);
    if prefix.is_empty() {
        return None;
    }
    if mode.is_visual() {
        visual_guide(&prefix)
    } else {
        normal_guide(&prefix)
    }
}

fn normalize_prefix(prefix: &str) -> String {
    prefix.chars().filter(|ch| !ch.is_ascii_digit()).collect()
}

fn normal_guide(prefix: &str) -> Option<Guide> {
    if let Some(hint) = dynamic_hint(prefix) {
        return Some(hint_guide(hint));
    }
    operator_guide(prefix)
        .or_else(|| g_guide(prefix))
        .or_else(|| z_guide(prefix, false))
        .or_else(|| bracket_guide(prefix))
}

fn visual_guide(prefix: &str) -> Option<Guide> {
    if let Some(hint) = dynamic_hint(prefix) {
        return Some(hint_guide(hint));
    }
    match prefix {
        "g" => Some(rows_guide(command_rows(VISUAL_G_COMMANDS))),
        "z" => z_guide(prefix, true),
        "i" => Some(rows_guide(object_rows(true))),
        "a" => Some(rows_guide(object_rows(false))),
        _ => None,
    }
}

fn operator_guide(prefix: &str) -> Option<Guide> {
    let mut chars = prefix.chars();
    let operator = chars.next()?;
    if !matches!(operator, 'd' | 'c' | 'y' | '>' | '<' | '=') {
        return None;
    }
    let rest = chars.as_str();
    if rest.is_empty() {
        return Some(rows_guide(operator_rows(operator)));
    }
    if matches!(operator, 'd' | 'c' | 'y') && matches!(rest, "i" | "a") {
        return Some(rows_guide(object_rows(rest == "i")));
    }
    if matches!(operator, 'd' | 'c' | 'y') && rest == "g" {
        return Some(rows_guide(label_rows(OPERATOR_G_MOTIONS)));
    }
    if matches!(operator, '>' | '<' | '=') && rest == "g" {
        return Some(rows_guide(label_rows(CASE_G_MOTIONS)));
    }
    None
}

fn g_guide(prefix: &str) -> Option<Guide> {
    match prefix {
        "g" => Some(rows_guide(command_rows(G_COMMANDS))),
        "g~" | "gu" | "gU" => Some(rows_guide(case_motion_rows(prefix))),
        "gc" => Some(rows_guide(label_rows(COMMENT_MOTIONS))),
        "g~i" | "gui" | "gUi" | "gci" => Some(rows_guide(object_rows(true))),
        "g~a" | "gua" | "gUa" | "gca" => Some(rows_guide(object_rows(false))),
        "g~g" | "gug" | "gUg" | "gcg" => Some(rows_guide(label_rows(CASE_G_MOTIONS))),
        _ => None,
    }
}

fn z_guide(prefix: &str, visual: bool) -> Option<Guide> {
    if prefix != "z" {
        return None;
    }
    let mut rows = Vec::new();
    if visual {
        rows.push(row('f', "create fold"));
    }
    rows.extend(command_rows(FOLD_COMMANDS));
    rows.extend(command_rows(PLACE_COMMANDS));
    Some(rows_guide(rows))
}

fn bracket_guide(prefix: &str) -> Option<Guide> {
    if !matches!(prefix, "[" | "]") {
        return None;
    }
    let forward = prefix == "]";
    let rows = BRACKET_COMMANDS
        .iter()
        .map(|command| {
            let label = if forward {
                command.forward_label
            } else {
                command.backward_label
            };
            row(command.key, label)
        })
        .collect();
    Some(rows_guide(rows))
}

fn dynamic_hint(prefix: &str) -> Option<&'static str> {
    let last = prefix.chars().last()?;
    if matches!(last, 'f' | 'F' | 't' | 'T') {
        return Some("target character");
    }
    if last == 'r' {
        return Some("replacement character");
    }
    if matches!(last, '\'' | '`') {
        return Some("mark name");
    }
    match prefix {
        "\"" => Some("register"),
        "@" | "q" => Some("macro register"),
        "m" => Some("mark name"),
        _ => None,
    }
}

fn operator_rows(operator: char) -> Vec<GuideRow> {
    let mut rows = Vec::new();
    for entry in OPERATOR_SEQUENCES {
        let Some(rest) = entry.keys.strip_prefix(operator) else {
            continue;
        };
        let mut chars = rest.chars();
        let Some(key) = chars.next() else {
            continue;
        };
        let label = if chars.next().is_some() {
            grouped_operator_label(key)
        } else {
            entry.label
        };
        push_unique(&mut rows, row(key, label));
    }
    rows
}

fn grouped_operator_label(key: char) -> &'static str {
    match key {
        'g' => "more motions",
        'i' => "inner text object",
        'a' => "around text object",
        _ => "motion",
    }
}

fn object_rows(inner: bool) -> Vec<GuideRow> {
    let word = if inner { "inner word" } else { "around word" };
    let mut rows = vec![row('w', word)];
    rows.extend(OBJECT_TARGETS.iter().map(|target| GuideRow {
        key: target.key,
        aliases: target.aliases,
        description: if inner {
            target.inner_label
        } else {
            target.around_label
        },
    }));
    rows
}

fn case_motion_rows(prefix: &str) -> Vec<GuideRow> {
    let mut rows = Vec::new();
    for motion in CASE_MOTIONS {
        if matches!(motion.key, '~' | 'u' | 'U') {
            continue;
        }
        rows.push(row(motion.key, motion.label));
    }
    if let Some(line_key) = prefix.chars().nth(1) {
        rows.push(row(line_key, "current line"));
    }
    rows
}

fn command_rows(commands: &[commands::KeyCommand]) -> Vec<GuideRow> {
    commands
        .iter()
        .map(|command| row(command.key, command.label))
        .collect()
}

fn label_rows(labels: &[commands::CommandLabel]) -> Vec<GuideRow> {
    labels
        .iter()
        .map(|label| row(label.key, label.label))
        .collect()
}

fn hint_guide(hint: &'static str) -> Guide {
    Guide {
        rows: Vec::new(),
        hint: Some(hint),
    }
}

fn rows_guide(rows: Vec<GuideRow>) -> Guide {
    Guide { rows, hint: None }
}

fn row(key: char, description: &'static str) -> GuideRow {
    GuideRow {
        key,
        aliases: &[],
        description,
    }
}

fn push_unique(rows: &mut Vec<GuideRow>, candidate: GuideRow) {
    if rows.iter().any(|row| row.key == candidate.key) {
        return;
    }
    rows.push(candidate);
}
