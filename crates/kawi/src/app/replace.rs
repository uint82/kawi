use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::{
    selection_at, wrapped_selection, Picker, PickerOutcome, ProjectRequest, ProjectResult,
    ProjectService, ReplacePreview, TextInput,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceStage {
    Search,
    Replacement,
    Preview,
}

#[derive(Debug, Clone)]
pub enum ReplaceOutcome {
    Consumed,
    Cancel,
    Apply(ReplacePreview),
}

pub struct ReplaceSession {
    picker: Picker,
    replacement: TextInput,
    stage: ReplaceStage,
    service: ProjectService,
    root: PathBuf,
    latest: u64,
    preview: Option<ReplacePreview>,
    search_rows: Vec<String>,
    preview_rows: Vec<String>,
    selected: Option<usize>,
    loading: bool,
    status: Option<String>,
}

impl ReplaceSession {
    pub fn new(root: PathBuf, wake: Sender<()>) -> Self {
        Self {
            picker: Picker::grep(),
            replacement: TextInput::new(),
            stage: ReplaceStage::Search,
            service: ProjectService::with_wake(wake),
            root,
            latest: 0,
            preview: None,
            search_rows: Vec::new(),
            preview_rows: Vec::new(),
            selected: None,
            loading: false,
            status: None,
        }
    }

    pub fn from_query(root: PathBuf, query: &str, wake: Sender<()>) -> Self {
        let mut session = Self::new(root, wake);
        session.picker.set_query(query);
        session.picker.set_loading(false);
        session.stage = ReplaceStage::Replacement;
        session
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn stage(&self) -> ReplaceStage {
        self.stage
    }

    pub fn query(&self) -> &str {
        self.picker.query()
    }

    pub fn replacement(&self) -> &str {
        self.replacement.text()
    }

    pub fn cursor(&self) -> usize {
        match self.stage {
            ReplaceStage::Search => self.picker.cursor(),
            ReplaceStage::Replacement | ReplaceStage::Preview => self.replacement.cursor(),
        }
    }

    pub fn rows(&self) -> &[String] {
        match self.stage {
            ReplaceStage::Preview => &self.preview_rows,
            ReplaceStage::Search | ReplaceStage::Replacement => &self.search_rows,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        match self.stage {
            ReplaceStage::Preview => self.selected,
            ReplaceStage::Search | ReplaceStage::Replacement => self.picker.selected(),
        }
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn matches(&self) -> usize {
        self.preview.as_ref().map_or(0, |preview| preview.matches)
    }

    pub fn files(&self) -> usize {
        self.preview
            .as_ref()
            .map_or(0, |preview| preview.files.len())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ReplaceOutcome {
        match self.stage {
            ReplaceStage::Search => self.handle_search_key(key),
            ReplaceStage::Replacement => self.handle_replacement_key(key),
            ReplaceStage::Preview => self.handle_preview_key(key),
        }
    }

    pub fn paste(&mut self, text: &str) {
        match self.stage {
            ReplaceStage::Search => {
                self.status = None;
                self.picker.insert_text(text);
                self.submit_search();
                self.sync_search_rows();
            }
            ReplaceStage::Replacement => {
                self.replacement.insert_text(text);
                self.status = None;
            }
            ReplaceStage::Preview => {}
        }
    }

    pub fn refresh(&mut self) -> bool {
        let mut changed = false;
        while let Some(result) = self.service.try_recv() {
            if result.id() != self.latest {
                continue;
            }
            self.apply(result);
            changed = true;
        }
        changed
    }

    pub fn set_status(&mut self, status: String) {
        self.status = Some(status);
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> ReplaceOutcome {
        if is_cancel_key(key) {
            return ReplaceOutcome::Cancel;
        }
        if is_replace_key(key) || key.code == KeyCode::Enter {
            self.begin_replacement();
            return ReplaceOutcome::Consumed;
        }
        let outcome = self.picker.handle_key(key);
        if outcome == PickerOutcome::QueryChanged {
            self.status = None;
            self.submit_search();
        }
        self.sync_search_rows();
        ReplaceOutcome::Consumed
    }

    fn handle_replacement_key(&mut self, key: KeyEvent) -> ReplaceOutcome {
        if is_cancel_key(key) {
            return ReplaceOutcome::Cancel;
        }
        if !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            self.status = None;
        }
        match key.code {
            KeyCode::Esc => {
                self.stage = ReplaceStage::Search;
                self.status = None;
            }
            KeyCode::Enter => self.submit_preview(),
            KeyCode::Backspace => self.replacement.backspace(),
            KeyCode::Delete => self.replacement.delete(),
            KeyCode::Left => self.replacement.move_left(),
            KeyCode::Right => self.replacement.move_right(),
            KeyCode::Home => self.replacement.move_home(),
            KeyCode::End => self.replacement.move_end(),
            KeyCode::Char(character) => self.edit_character(key, character),
            _ => {}
        }
        ReplaceOutcome::Consumed
    }

    fn handle_preview_key(&mut self, key: KeyEvent) -> ReplaceOutcome {
        if is_cancel_key(key) {
            return ReplaceOutcome::Cancel;
        }
        match key.code {
            KeyCode::Esc => {
                self.stage = ReplaceStage::Replacement;
                self.status = None;
            }
            KeyCode::Char('n') if is_plain(key) => {
                self.stage = ReplaceStage::Replacement;
                self.status = None;
            }
            KeyCode::Char('y') if is_plain(key) => {
                if let Some(preview) = self.preview.clone() {
                    return ReplaceOutcome::Apply(preview);
                }
            }
            KeyCode::Up => self.navigate(false, 1),
            KeyCode::Down => self.navigate(true, 1),
            KeyCode::PageUp => self.navigate(false, 10),
            KeyCode::PageDown => self.navigate(true, 10),
            KeyCode::Home => self.select(0),
            KeyCode::End => self.select(self.preview_rows.len().saturating_sub(1)),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => self.navigate(false, 1),
            KeyCode::BackTab => self.navigate(false, 1),
            KeyCode::Tab => self.navigate(true, 1),
            _ => {}
        }
        ReplaceOutcome::Consumed
    }

    fn sync_search_rows(&mut self) {
        self.search_rows = self
            .picker
            .rows()
            .iter()
            .map(|row| row.display.clone())
            .collect();
    }

    fn edit_character(&mut self, key: KeyEvent, character: char) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match character {
                'u' => self.replacement.clear(),
                'w' => self.replacement.delete_word(),
                _ => {}
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return;
        }
        self.replacement.insert_char(character);
        self.status = None;
    }

    fn begin_replacement(&mut self) {
        if self.picker.query().is_empty() {
            self.status = Some("Type a search pattern first".to_string());
            return;
        }
        self.stage = ReplaceStage::Replacement;
        self.status = None;
    }

    fn submit_search(&mut self) {
        let query = self.picker.query().to_string();
        if query.is_empty() {
            self.picker.clear_hits();
            self.loading = false;
            self.search_rows.clear();
            self.selected = None;
            return;
        }
        self.latest = self.service.submit(ProjectRequest::Grep {
            root: self.root.clone(),
            query,
            limit: 2_000,
        });
        self.picker.set_loading(true);
        self.loading = true;
        self.status = None;
    }

    fn submit_preview(&mut self) {
        let query = self.picker.query().to_string();
        if query.is_empty() {
            self.status = Some("Type a search pattern first".to_string());
            return;
        }
        self.preview = None;
        self.preview_rows.clear();
        self.selected = None;
        self.latest = self.service.submit(ProjectRequest::Replace {
            root: self.root.clone(),
            query,
            replacement: self.replacement.text().to_string(),
            limit: kawi_picker::MAX_REPLACE_MATCHES,
        });
        self.loading = true;
        self.status = None;
    }

    fn apply(&mut self, result: ProjectResult) {
        match result {
            ProjectResult::Grep { hits, error, .. } => {
                self.loading = false;
                if let Some(error) = error {
                    self.picker.set_error(super::compact_error(&error));
                } else {
                    self.picker.set_hits(hits);
                }
                self.sync_search_rows();
            }
            ProjectResult::Replace { preview, error, .. } => {
                self.loading = false;
                if self.stage == ReplaceStage::Search {
                    return;
                }
                if let Some(error) = error {
                    self.status = Some(super::compact_error(&error));
                    self.stage = ReplaceStage::Replacement;
                    return;
                }
                let Some(preview) = preview else {
                    self.status = Some("No preview".to_string());
                    self.stage = ReplaceStage::Replacement;
                    return;
                };
                self.preview_rows = preview_rows(&preview);
                self.selected = if self.preview_rows.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.preview = Some(preview);
                self.stage = ReplaceStage::Preview;
                self.status = None;
            }
            ProjectResult::Files { .. } => {}
        }
    }

    fn navigate(&mut self, forward: bool, amount: usize) {
        self.selected = wrapped_selection(self.selected, self.preview_rows.len(), forward, amount);
    }

    fn select(&mut self, index: usize) {
        self.selected = selection_at(index, self.preview_rows.len());
    }
}

pub(super) fn is_replace_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R'))
}

fn display_text(text: &str) -> String {
    let mut result = String::new();
    for character in text.chars() {
        match character {
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                result.extend(character.escape_default());
            }
            character => result.push(character),
        }
    }
    result
}

fn is_cancel_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')))
}

fn is_plain(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn preview_rows(preview: &ReplacePreview) -> Vec<String> {
    let mut rows = Vec::new();
    for file in &preview.files {
        rows.push(display_text(&file.path));
        for entry in preview
            .entries
            .iter()
            .filter(|entry| entry.path == file.path)
        {
            rows.push(format!("- {}: {}", entry.line, display_text(&entry.before)));
            rows.push(format!("+ {}: {}", entry.line, display_text(&entry.after)));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use super::*;

    fn wake() -> Sender<()> {
        let (sender, _) = std::sync::mpsc::channel();
        sender
    }

    fn project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_replace_session_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("project");
        root
    }

    fn wait_for_preview(session: &mut ReplaceSession) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            session.refresh();
            if session.stage() == ReplaceStage::Preview {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("preview timed out: {:?}", session.status());
    }

    #[test]
    fn replacement_flow_previews_and_applies() {
        let root = project("flow");
        fs::write(root.join("a.txt"), "foo foo\n").expect("file");
        let mut session = ReplaceSession::from_query(root.clone(), "foo", wake());
        for character in "bar".chars() {
            session.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        session.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        wait_for_preview(&mut session);
        assert_eq!(session.matches(), 2);
        assert_eq!(session.files(), 1);
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
            ReplaceOutcome::Apply(_)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preview_rows_show_file_and_diff_lines() {
        let preview = ReplacePreview {
            matches: 1,
            files: vec![kawi_picker::ReplaceFile {
                path: "a.txt".to_string(),
                content: "bar\n".to_string(),
                original_hash: 0,
            }],
            entries: vec![kawi_picker::ReplaceEntry {
                path: "a.txt".to_string(),
                line: 1,
                before: "foo".to_string(),
                after: "bar".to_string(),
            }],
        };
        assert_eq!(
            preview_rows(&preview),
            vec!["a.txt", "- 1: foo", "+ 1: bar"]
        );
    }

    #[test]
    fn preview_rows_escape_control_characters() {
        let preview = ReplacePreview {
            matches: 1,
            files: vec![kawi_picker::ReplaceFile {
                path: "a.txt".to_string(),
                content: "x\ty\n".to_string(),
                original_hash: 0,
            }],
            entries: vec![kawi_picker::ReplaceEntry {
                path: "a.txt".to_string(),
                line: 1,
                before: "a\tb".to_string(),
                after: "x\ty".to_string(),
            }],
        };
        assert_eq!(
            preview_rows(&preview),
            vec!["a.txt", "- 1: a\\tb", "+ 1: x\\ty"]
        );
    }

    #[test]
    fn cancelling_the_preview_does_not_write_files() {
        let root = project("cancel_preview");
        fs::write(root.join("a.txt"), "foo\n").expect("file");
        let mut session = ReplaceSession::from_query(root.clone(), "foo", wake());
        session.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        session.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        wait_for_preview(&mut session);
        session.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(session.stage(), ReplaceStage::Replacement);
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).expect("disk"),
            "foo\n"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn control_c_cancels_every_stage() {
        let root = project("cancel");
        let mut session = ReplaceSession::from_query(root.clone(), "foo", wake());
        assert!(matches!(
            session.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            ReplaceOutcome::Cancel
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
