use super::{Candidate, Picker, PickerKind, PickerRow, MAX_ROWS};
use crate::fuzzy::Query;

impl Picker {
    pub(super) fn refresh(&mut self) {
        match self.kind {
            PickerKind::Files => self.refresh_files(),
            PickerKind::Grep => self.refresh_grep(),
            PickerKind::Buffers => self.refresh_buffers(),
            PickerKind::Symbols => self.refresh_locations(),
            PickerKind::WorkspaceSymbols => {
                self.rows = self
                    .locations
                    .iter()
                    .take(MAX_ROWS)
                    .map(|candidate| candidate.row.clone())
                    .collect();
            }
            PickerKind::Locations
            | PickerKind::GitBranches
            | PickerKind::GitLog
            | PickerKind::GitCommitFiles => self.refresh_locations(),
        }
        self.selected = if self.rows.is_empty() { None } else { Some(0) };
    }

    fn refresh_files(&mut self) {
        self.rows = self.rank_fuzzy(self.files.iter());
    }

    fn refresh_grep(&mut self) {
        self.rows = self
            .hits
            .iter()
            .take(MAX_ROWS)
            .map(|hit| PickerRow {
                display: format!("{}:{}:{}: {}", hit.path, hit.line, hit.column, hit.text),
                path: hit.path.clone(),
                line: Some(hit.line),
                column: Some(hit.column),
                buffer: None,
                secondary: None,
                deprecated: false,
                id: None,
            })
            .collect();
    }

    fn refresh_locations(&mut self) {
        self.rows = self.location_rows();
    }

    pub(super) fn location_rows(&self) -> Vec<PickerRow> {
        if self.query().is_empty() {
            return self
                .locations
                .iter()
                .take(MAX_ROWS)
                .map(|candidate| candidate.row.clone())
                .collect();
        }
        self.rank_fuzzy(self.locations.iter())
    }

    fn refresh_buffers(&mut self) {
        self.rows = self.rank_fuzzy(self.buffers.iter());
    }

    fn rank_fuzzy<'a>(&self, candidates: impl Iterator<Item = &'a Candidate>) -> Vec<PickerRow> {
        let query = Query::new(self.query());
        if query.is_empty() {
            let mut candidates: Vec<&Candidate> = candidates.collect();
            candidates.sort_by(|left, right| left.row.display.cmp(&right.row.display));
            return candidates
                .into_iter()
                .take(MAX_ROWS)
                .map(|candidate| candidate.row.clone())
                .collect();
        }
        let mut ranked: Vec<(i32, &Candidate)> = candidates
            .filter_map(|candidate| {
                query
                    .score(&candidate.row.display, &candidate.folded)
                    .map(|score| (score, candidate))
            })
            .collect();
        let compare = |left: &(i32, &Candidate), right: &(i32, &Candidate)| {
            right
                .0
                .cmp(&left.0)
                .then(left.1.row.display.cmp(&right.1.row.display))
        };
        if ranked.len() > MAX_ROWS {
            ranked.select_nth_unstable_by(MAX_ROWS, compare);
            ranked.truncate(MAX_ROWS);
        }
        ranked.sort_by(compare);
        ranked
            .into_iter()
            .map(|(_, candidate)| candidate.row.clone())
            .collect()
    }
}

impl Candidate {
    pub(super) fn new(row: PickerRow) -> Self {
        let folded = row.display.to_lowercase();
        Self { row, folded }
    }
}

pub(super) fn file_candidate(path: String) -> Candidate {
    Candidate::new(PickerRow {
        display: path.clone(),
        path,
        line: None,
        column: None,
        buffer: None,
        secondary: None,
        deprecated: false,
        id: None,
    })
}

pub(super) fn buffer_candidate((number, name): (usize, String)) -> Candidate {
    Candidate::new(PickerRow {
        display: format!("{number}: {name}"),
        path: name,
        line: None,
        column: None,
        buffer: Some(number),
        secondary: None,
        deprecated: false,
        id: None,
    })
}
