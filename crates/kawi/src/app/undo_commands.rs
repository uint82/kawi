use anyhow::Result;

use kawi_buffer::UndoLeaf;
use kawi_editor::command::UndoCommand;

use super::undo_tree::undo_boundary;
use super::App;

const MILLIS_PER_SECOND: u64 = 1000;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;

impl App {
    pub(super) fn handle_undo_command(&mut self, command: UndoCommand) -> Result<bool> {
        match command {
            UndoCommand::Earlier(offset) => {
                if !self.editor.jump_undo_offset(offset, false) {
                    self.show_message(undo_boundary(false).to_string());
                }
            }
            UndoCommand::Later(offset) => {
                if !self.editor.jump_undo_offset(offset, true) {
                    self.show_message(undo_boundary(true).to_string());
                }
            }
            UndoCommand::Jump(sequence) => {
                if !self.editor.jump_undo_sequence(sequence) {
                    self.show_message(format!("E830: Undo number {sequence} not found"));
                }
            }
            UndoCommand::List => self.show_undo_list(),
        }
        self.reveal_cursor();
        Ok(false)
    }

    fn show_undo_list(&mut self) {
        let leaves = self.editor.undo_leaves();
        if leaves.is_empty() {
            self.show_message("No undo states".to_string());
            return;
        }
        self.show_pager(format_undo_list(&leaves));
    }
}

fn format_undo_list(leaves: &[UndoLeaf]) -> Vec<String> {
    let mut lines = Vec::with_capacity(leaves.len() + 1);
    lines.push(format!(
        "{:>6} {:>7}  {:<15}  {:>5}",
        "number", "changes", "when", "saved"
    ));
    for leaf in leaves {
        lines.push(format!(
            "{}{:>6} {:>7}  {:<15}  {:>5}",
            if leaf.current { ">" } else { " " },
            leaf.sequence,
            leaf.changes,
            format_age(leaf.age_millis),
            leaf.save_marker
                .map_or_else(|| "-".to_string(), |marker| marker.to_string())
        ));
    }
    lines
}

fn format_age(age_millis: u64) -> String {
    let seconds = age_millis / MILLIS_PER_SECOND;
    if seconds < SECONDS_PER_MINUTE {
        return format_units(seconds, "seconds");
    }
    let minutes = seconds / SECONDS_PER_MINUTE;
    if minutes < SECONDS_PER_MINUTE {
        return format_units(minutes, "minutes");
    }
    let hours = seconds / SECONDS_PER_HOUR;
    if hours < SECONDS_PER_DAY {
        return format_units(hours, "hours");
    }
    format_units(seconds / SECONDS_PER_DAY, "days")
}

fn format_units(value: u64, unit: &str) -> String {
    let unit = if value == 1 {
        unit.trim_end_matches('s')
    } else {
        unit
    };
    format!("{value} {unit} ago")
}

#[cfg(test)]
mod tests;
