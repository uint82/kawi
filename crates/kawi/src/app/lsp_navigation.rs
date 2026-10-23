use std::path::PathBuf;

use kawi_editor::Editor;
use kawi_lsp::{CursorPosition, HoverEvent, LocationEvent, LspService, TextLocation};

use super::lsp::editor_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocationKind {
    Definition,
    References,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigationRequest {
    Definition,
    References,
    Hover,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocationResult {
    pub(super) kind: LocationKind,
    pub(super) locations: Vec<TextLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HoverResult {
    pub(super) lines: Vec<String>,
}

#[derive(Default)]
pub(super) struct LspNavigation {
    path: Option<PathBuf>,
    revision: Option<u64>,
    cursor: Option<(u32, u32)>,
    pending: Option<PendingRequest>,
    locations: Option<LocationResult>,
    hover: Option<HoverResult>,
}

struct PendingRequest {
    request: u64,
    kind: NavigationRequest,
}

impl LspNavigation {
    pub(super) fn request(
        &mut self,
        request: NavigationRequest,
        editor: &Editor,
        service: Option<&LspService>,
    ) -> Option<String> {
        let Some(service) = service else {
            return Some("lsp unavailable".to_string());
        };
        let Some(path) = editor_path(editor) else {
            return Some("E32: No file name".to_string());
        };
        self.cancel(Some(service));
        let position = cursor_position(editor);
        let id = match request {
            NavigationRequest::Definition => service.definition(&path, position),
            NavigationRequest::References => service.references(&path, position),
            NavigationRequest::Hover => service.hover(&path, position),
        };
        self.path = Some(path);
        self.revision = Some(editor.buffer_revision());
        self.cursor = Some((position.line, position.column));
        self.pending = Some(PendingRequest {
            request: id,
            kind: request,
        });
        self.locations = None;
        self.hover = None;
        None
    }

    pub(super) fn sync_editor(&mut self, editor: &Editor, service: Option<&LspService>) {
        let path = editor_path(editor);
        let revision = Some(editor.buffer_revision());
        let position = cursor_position(editor);
        let cursor = Some((position.line, position.column));
        if path == self.path && revision == self.revision && cursor == self.cursor {
            return;
        }
        self.cancel(service);
        self.locations = None;
        self.hover = None;
        self.path = path;
        self.revision = revision;
        self.cursor = cursor;
    }

    pub(super) fn cancel(&mut self, service: Option<&LspService>) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        if let Some(service) = service {
            service.cancel(pending.request);
        }
        true
    }

    pub(super) fn apply_location(&mut self, event: &LocationEvent, kind: LocationKind) -> bool {
        if !self.matches(event.request, location_request(kind)) {
            return false;
        }
        self.pending = None;
        self.locations = Some(LocationResult {
            kind,
            locations: event.locations.clone(),
        });
        true
    }

    pub(super) fn apply_hover(&mut self, event: &HoverEvent) -> bool {
        if !self.matches(event.request, NavigationRequest::Hover) {
            return false;
        }
        self.pending = None;
        self.hover = Some(HoverResult {
            lines: event.contents.clone(),
        });
        true
    }

    pub(super) fn apply_error(&mut self, request: u64) -> bool {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.request != request)
        {
            return false;
        }
        self.pending = None;
        true
    }

    pub(super) fn take_location(&mut self) -> Option<LocationResult> {
        self.locations.take()
    }

    pub(super) fn take_hover(&mut self) -> Option<HoverResult> {
        self.hover.take()
    }

    fn matches(&self, request: u64, kind: NavigationRequest) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| pending.request == request && pending.kind == kind)
    }
}

fn location_request(kind: LocationKind) -> NavigationRequest {
    match kind {
        LocationKind::Definition => NavigationRequest::Definition,
        LocationKind::References => NavigationRequest::References,
    }
}

pub(super) fn cursor_position(editor: &Editor) -> CursorPosition {
    cursor(editor.row(), editor.col())
}

pub(super) fn cursor(row: usize, column: usize) -> CursorPosition {
    CursorPosition {
        line: u32::try_from(row).unwrap_or(u32::MAX),
        column: u32::try_from(column).unwrap_or(u32::MAX),
    }
}

impl super::App {
    pub(super) fn request_definition(&mut self) -> anyhow::Result<bool> {
        if let Some(message) = self.lsp.definition(&self.editor) {
            self.show_message(message);
        }
        Ok(false)
    }

    pub(super) fn request_references(&mut self) -> anyhow::Result<bool> {
        if let Some(message) = self.lsp.references(&self.editor) {
            self.show_message(message);
        }
        Ok(false)
    }

    pub(super) fn request_hover(&mut self) -> anyhow::Result<bool> {
        if let Some(message) = self.lsp.hover(&self.editor) {
            self.show_message(message);
        }
        Ok(false)
    }

    pub(super) fn handle_location_result(&mut self, result: LocationResult) {
        if self.picker.is_some() || self.replace.is_some() {
            return;
        }
        self.pager = None;
        if result.kind == LocationKind::Definition && result.locations.len() == 1 {
            let location = &result.locations[0];
            if let Err(message) = self.open_file_location(
                &location.path,
                Some(location.line as usize),
                Some(location.column as usize),
            ) {
                self.show_message(message);
            }
            return;
        }
        self.open_location_picker(result);
    }

    pub(super) fn handle_hover_result(&mut self, result: HoverResult) {
        if self.picker.is_some() || self.replace.is_some() {
            return;
        }
        let text = result.lines.join("\n");
        let lines = super::pager_lines(&text, self.editor.viewport().width());
        if lines.is_empty() {
            self.show_message("No hover information".to_string());
        } else {
            self.show_pager(lines);
        }
    }

    fn open_location_picker(&mut self, result: LocationResult) {
        let root = self.picker_root();
        let rows = result
            .locations
            .iter()
            .map(|location| location_row(&root, location))
            .collect();
        let title = match result.kind {
            LocationKind::Definition => "Definitions",
            LocationKind::References => "References",
        };
        self.picker = Some(super::picker::PickerSession::locations(root, title, rows));
    }

    pub(super) fn open_file_location(
        &mut self,
        path: &std::path::Path,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Result<(), String> {
        let current =
            self.editor.has_file() && same_file(std::path::Path::new(self.editor.filename()), path);
        if !current {
            let name = path.to_string_lossy().to_string();
            self.editor
                .edit_file(Some(&name), false)
                .map_err(super::file_error_message)?;
        }
        if let (Some(line), Some(column)) = (line, column) {
            self.editor.apply(kawi_editor::Action::JumpTo(line, column));
        }
        self.reveal_cursor();
        Ok(())
    }
}

fn location_row(root: &std::path::Path, location: &TextLocation) -> kawi_picker::PickerRow {
    let path = location.path.strip_prefix(root).unwrap_or(&location.path);
    kawi_picker::PickerRow {
        display: format!(
            "{}:{}:{}",
            path.display(),
            location.line.saturating_add(1),
            location.column.saturating_add(1)
        ),
        path: location.path.to_string_lossy().to_string(),
        line: Some(location.line.saturating_add(1) as usize),
        column: Some(location.column.saturating_add(1) as usize),
        buffer: None,
    }
}

fn same_file(left: &std::path::Path, right: &std::path::Path) -> bool {
    std::path::absolute(left).is_ok_and(|left| left == right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;
    use kawi_editor::Action;

    fn editor() -> Editor {
        Editor::new(Buffer::from_string("one\ntwo\nthree\n"))
    }

    fn pending(navigation: &mut LspNavigation, request: u64, kind: NavigationRequest) {
        navigation.pending = Some(PendingRequest { request, kind });
    }

    fn location_event(request: u64) -> LocationEvent {
        LocationEvent {
            request,
            path: PathBuf::from("/tmp/file.rs"),
            position: CursorPosition { line: 0, column: 1 },
            locations: Vec::new(),
        }
    }

    #[test]
    fn location_results_require_the_current_request() {
        let mut navigation = LspNavigation::default();
        pending(&mut navigation, 3, NavigationRequest::Definition);
        assert!(!navigation.apply_location(&location_event(2), LocationKind::Definition));
        assert!(navigation.apply_location(&location_event(3), LocationKind::Definition));
        assert!(navigation.take_location().is_some());
    }

    #[test]
    fn hover_results_require_the_current_request() {
        let mut navigation = LspNavigation::default();
        pending(&mut navigation, 4, NavigationRequest::Hover);
        let event = HoverEvent {
            request: 4,
            path: PathBuf::from("/tmp/file.rs"),
            position: CursorPosition { line: 0, column: 0 },
            contents: vec!["hover".to_string()],
        };
        assert!(navigation.apply_hover(&event));
        assert_eq!(
            navigation.take_hover(),
            Some(HoverResult {
                lines: vec!["hover".to_string()],
            })
        );
    }

    #[test]
    fn multiple_locations_open_a_picker() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_nav_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("main.rs");
        std::fs::write(&path, "one\ntwo\n").unwrap();
        let mut app = super::super::App::new(Buffer::from_string("one"), Flavor::Vim);
        app.handle_location_result(LocationResult {
            kind: LocationKind::References,
            locations: vec![
                TextLocation {
                    path: path.clone(),
                    line: 0,
                    column: 1,
                },
                TextLocation {
                    path: path.clone(),
                    line: 1,
                    column: 2,
                },
            ],
        });
        let picker = app.picker.as_ref().expect("picker");
        assert_eq!(picker.picker().kind(), kawi_picker::PickerKind::Locations);
        assert_eq!(picker.picker().title(), "References");
        assert_eq!(picker.picker().rows().len(), 2);
        let mut lines: Vec<usize> = picker
            .picker()
            .rows()
            .iter()
            .filter_map(|row| row.line)
            .collect();
        lines.sort_unstable();
        assert_eq!(lines, vec![1, 2]);
        let row = picker.picker().rows()[1].clone();
        let expected_line = row.line.expect("line").saturating_sub(1);
        let expected_column = row.column.expect("column").saturating_sub(1);
        app.open_picker_row(&row, &root).expect("open location");
        assert_eq!(app.editor.filename(), path.to_string_lossy());
        assert_eq!(app.editor.row(), expected_line);
        assert_eq!(app.editor.col(), expected_column);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_definition_opens_the_location() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_def_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("main.rs");
        std::fs::write(&path, "one\ntwo\n").unwrap();
        let mut app = super::super::App::new(Buffer::from_string("one"), Flavor::Vim);
        app.handle_location_result(LocationResult {
            kind: LocationKind::Definition,
            locations: vec![TextLocation {
                path: path.clone(),
                line: 1,
                column: 1,
            }],
        });
        assert_eq!(app.editor.filename(), path.to_string_lossy());
        assert_eq!(app.editor.row(), 1);
        assert_eq!(app.editor.col(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hover_opens_the_pager() {
        let mut app = super::super::App::new(Buffer::from_string("one"), Flavor::Vim);
        app.handle_hover_result(HoverResult {
            lines: vec!["fn fake()".to_string(), "Fake hover".to_string()],
        });
        assert!(app.pager.is_some());
    }

    #[test]
    fn lsp_keys_report_a_missing_service() {
        for keys in [
            vec![KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE)],
            vec![
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            ],
            vec![
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            ],
            vec![KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL)],
        ] {
            let mut app = super::super::App::new(Buffer::from_string("one"), Flavor::Vim);
            for key in keys {
                app.handle_key(key).expect("key");
            }
            assert_eq!(app.last_message(), Some("lsp unavailable"));
        }
    }

    #[test]
    fn cancellation_clears_a_pending_request() {
        let mut navigation = LspNavigation::default();
        pending(&mut navigation, 6, NavigationRequest::Hover);
        assert!(navigation.cancel(None));
        assert!(navigation.pending.is_none());
        assert!(!navigation.cancel(None));
    }

    #[test]
    fn cursor_changes_cancel_pending_requests() {
        let mut navigation = LspNavigation::default();
        let mut editor = editor();
        navigation.sync_editor(&editor, None);
        pending(&mut navigation, 5, NavigationRequest::Definition);
        editor.apply(Action::JumpTo(1, 0));
        navigation.sync_editor(&editor, None);
        assert!(navigation.pending.is_none());
    }
}
