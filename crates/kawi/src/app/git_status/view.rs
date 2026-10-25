use kawi_git::{EntryKind, Status, StatusEntry};

const CONFLICTS: &str = "Conflicts";
const STAGED: &str = "Staged";
const UNSTAGED: &str = "Unstaged";
const UNTRACKED: &str = "Untracked";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Section {
    Conflicts,
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Item {
    pub section: Section,
    pub entry: StatusEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Header(&'static str, usize),
    Entry(Item),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    section: Section,
    path: std::path::PathBuf,
}

pub(super) struct GitStatusView {
    rows: Vec<Row>,
    selected: usize,
    status: String,
    pending_discard: Option<usize>,
}

impl GitStatusView {
    pub(super) fn new(status: &Status, selected: Option<Key>) -> Self {
        let rows = build_rows(status);
        let selected = selected
            .and_then(|key| rows.iter().position(|row| row_matches(row, &key)))
            .or_else(|| rows.iter().position(|row| matches!(row, Row::Entry(_))))
            .unwrap_or(0);
        Self {
            rows,
            selected,
            status: format!(
                "{} {}",
                status.entries.len(),
                if status.entries.len() == 1 {
                    "entry"
                } else {
                    "entries"
                }
            ),
            pending_discard: None,
        }
    }

    pub(super) fn lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|row| match row {
                Row::Header(label, count) => format!("-- {label} ({count}) --"),
                Row::Entry(item) => item_line(item),
            })
            .collect()
    }

    pub(super) fn selected(&self) -> Option<usize> {
        matches!(self.rows.get(self.selected), Some(Row::Entry(_))).then_some(self.selected)
    }

    pub(super) fn selected_item(&self) -> Option<&Item> {
        match self.rows.get(self.selected) {
            Some(Row::Entry(item)) => Some(item),
            _ => None,
        }
    }

    pub(super) fn selected_key(&self) -> Option<Key> {
        self.selected_item().map(|item| Key {
            section: item.section.clone(),
            path: item.entry.path.clone(),
        })
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }

    pub(super) fn move_next(&mut self) {
        self.move_selection(true);
    }

    pub(super) fn move_previous(&mut self) {
        self.move_selection(false);
    }

    pub(super) fn move_first(&mut self) {
        if let Some(index) = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Entry(_)))
        {
            self.selected = index;
        }
    }

    pub(super) fn move_last(&mut self) {
        if let Some(index) = self
            .rows
            .iter()
            .rposition(|row| matches!(row, Row::Entry(_)))
        {
            self.selected = index;
        }
    }

    pub(super) fn pending_discard(&self) -> Option<usize> {
        self.pending_discard
    }

    pub(super) fn request_discard(&mut self) -> Option<Item> {
        let item = self.selected_item()?.clone();
        if item.section == Section::Staged || item.section == Section::Conflicts {
            return None;
        }
        self.pending_discard = Some(self.selected);
        Some(item)
    }

    pub(super) fn cancel_discard(&mut self) {
        self.pending_discard = None;
    }

    pub(super) fn take_discard(&mut self) -> Option<Item> {
        let row = self.pending_discard.take()?;
        match self.rows.get(row) {
            Some(Row::Entry(item)) => Some(item.clone()),
            _ => None,
        }
    }

    fn move_selection(&mut self, forward: bool) {
        if self.rows.is_empty() {
            return;
        }
        let mut index = self.selected;
        loop {
            index = if forward {
                (index + 1) % self.rows.len()
            } else if index == 0 {
                self.rows.len() - 1
            } else {
                index - 1
            };
            if matches!(self.rows.get(index), Some(Row::Entry(_))) {
                self.selected = index;
                return;
            }
            if index == self.selected {
                return;
            }
        }
    }
}

fn build_rows(status: &Status) -> Vec<Row> {
    let mut rows = Vec::new();
    push_section(
        &mut rows,
        CONFLICTS,
        status
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Unmerged),
        Section::Conflicts,
    );
    push_section(
        &mut rows,
        STAGED,
        status.entries.iter().filter(|entry| {
            entry.kind != EntryKind::Unmerged
                && entry.kind != EntryKind::Untracked
                && entry.has_staged()
        }),
        Section::Staged,
    );
    push_section(
        &mut rows,
        UNSTAGED,
        status.entries.iter().filter(|entry| {
            entry.kind != EntryKind::Unmerged
                && entry.kind != EntryKind::Untracked
                && entry.has_unstaged()
        }),
        Section::Unstaged,
    );
    push_section(
        &mut rows,
        UNTRACKED,
        status
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Untracked),
        Section::Untracked,
    );
    rows
}

fn push_section<'a>(
    rows: &mut Vec<Row>,
    label: &'static str,
    entries: impl Iterator<Item = &'a StatusEntry>,
    section: Section,
) {
    let entries: Vec<StatusEntry> = entries.cloned().collect();
    if entries.is_empty() {
        return;
    }
    rows.push(Row::Header(label, entries.len()));
    rows.extend(entries.into_iter().map(|entry| {
        Row::Entry(Item {
            section: section.clone(),
            entry,
        })
    }));
}

fn item_line(item: &Item) -> String {
    let entry = &item.entry;
    let mut path = entry.path.display().to_string();
    if let Some(original) = entry.original_path.as_ref() {
        path.push_str(" <- ");
        path.push_str(&original.display().to_string());
    }
    format!("{}  {}", entry.code(), path)
}

fn row_matches(row: &Row, key: &Key) -> bool {
    match row {
        Row::Entry(item) => item.section == key.section && item.entry.path == key.path,
        Row::Header(_, _) => false,
    }
}

#[cfg(test)]
mod tests;
