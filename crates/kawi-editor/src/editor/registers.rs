use super::Editor;
use crate::clipboard::ClipboardKind;
use crate::register::{Register, RegisterKind};
use crate::registers::RegisterSlot;

impl Editor {
    pub fn set_clipboard(&mut self, clipboard: Box<dyn crate::clipboard::Clipboard>) {
        self.clipboard = Some(clipboard);
    }

    // TODO(registers): support the read-only registers (". "% ": "/ "#) and the expression register.
    pub(super) fn select_register(&mut self, name: char) {
        if !matches!(name, 'a'..='z' | 'A'..='Z' | '0'..='9' | '"' | '-' | '+' | '*' | '_') {
            return;
        }
        self.begin_sequence();
        self.pending_register = Some(name);
    }

    pub(super) fn write_yank(&mut self, kind: RegisterKind, text: String) {
        if self.pending_register == Some('_') {
            return;
        }
        let register = Register::from_parts(text, kind);
        let slot = self.registers.yank(self.pending_register, register);
        self.sync_clipboard(slot);
        if named_register_is_unnamed(self.pending_register) {
            self.mirror_unnamed_clipboard();
        }
    }

    pub(super) fn write_delete(
        &mut self,
        kind: RegisterKind,
        text: String,
        line_count: usize,
        linewise: bool,
    ) {
        let register = Register::from_parts(text, kind);
        let slot = self.registers.delete(
            self.pending_register,
            register,
            line_count,
            linewise,
            self.force_numbered_register,
        );
        if let Some(slot) = slot {
            self.sync_clipboard(slot);
        }
        if named_register_is_unnamed(self.pending_register) {
            self.mirror_unnamed_clipboard();
        }
    }

    pub(super) fn capture_delete_range(&mut self, start: (usize, usize), end: (usize, usize)) {
        let text = self.buffer.range_text(start.0, start.1, end.0, end.1);
        let line_count = end.0.saturating_sub(start.0) + 1;
        self.write_delete(RegisterKind::CharWise, text, line_count, false);
    }

    pub(super) fn capture_delete_span(&mut self, start: (usize, usize), end: (usize, usize)) {
        let text = self.buffer.span_text(start.0, start.1, end.0, end.1);
        let line_count = end.0.saturating_sub(start.0) + 1;
        self.write_delete(RegisterKind::CharWise, text, line_count, false);
    }

    pub(super) fn insert_register(&mut self, name: char) {
        let Some(register) = self.read_register(Some(name)) else {
            return;
        };
        let text = clipboard_text(&register);
        for ch in text.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(ch);
            }
        }
    }

    pub(super) fn read_register(&mut self, name: Option<char>) -> Option<Register> {
        match name {
            Some('_') => None,
            Some('+') => self.read_clipboard(RegisterSlot::Plus),
            Some('*') => self.read_clipboard(RegisterSlot::Star),
            None | Some('"') => self.unnamed_register(),
            Some(other) => self
                .registers
                .slot(other)
                .and_then(|slot| self.registers.get(slot).cloned()),
        }
    }

    fn unnamed_register(&mut self) -> Option<Register> {
        if self.clipboard_unnamed {
            return self.read_clipboard(RegisterSlot::Plus);
        }
        self.unnamed_register_internal()
    }

    fn unnamed_register_internal(&self) -> Option<Register> {
        self.registers
            .unnamed_slot()
            .and_then(|slot| self.registers.get(slot).cloned())
    }

    fn read_clipboard(&mut self, slot: RegisterSlot) -> Option<Register> {
        let cached = self.registers.get(slot).cloned();
        match self.fetch_clipboard(slot) {
            Some(fetched) => {
                let register = match cached {
                    Some(cached) if same_clipboard_text(&cached, &fetched) => cached,
                    _ => fetched,
                };
                self.registers.store_clipboard(slot, register.clone());
                Some(register)
            }
            None => cached.or_else(|| self.unnamed_register_internal()),
        }
    }

    fn fetch_clipboard(&mut self, slot: RegisterSlot) -> Option<Register> {
        let kind = clipboard_kind(slot);
        let text = self.clipboard.as_mut()?.read(kind)?;
        if text.is_empty() {
            return None;
        }
        Some(clipboard_register(text))
    }

    fn mirror_unnamed_clipboard(&mut self) {
        if !self.clipboard_unnamed {
            return;
        }
        let Some(slot) = self.registers.unnamed_slot() else {
            return;
        };
        let Some(register) = self.registers.get(slot).cloned() else {
            return;
        };
        self.registers.store_clipboard(RegisterSlot::Plus, register);
        self.sync_clipboard(RegisterSlot::Plus);
    }

    fn sync_clipboard(&mut self, slot: RegisterSlot) {
        let kind = match slot {
            RegisterSlot::Star => ClipboardKind::Primary,
            RegisterSlot::Plus => ClipboardKind::Clipboard,
            _ => return,
        };
        let Some(register) = self.registers.get(slot) else {
            return;
        };
        let text = clipboard_text(register);
        if let Some(clipboard) = self.clipboard.as_mut() {
            clipboard.write(kind, &text);
        }
    }
}

fn same_clipboard_text(cached: &Register, fetched: &Register) -> bool {
    clipboard_text(cached) == clipboard_text(fetched)
}

fn named_register_is_unnamed(name: Option<char>) -> bool {
    matches!(name, None | Some('"'))
}

fn clipboard_kind(slot: RegisterSlot) -> ClipboardKind {
    match slot {
        RegisterSlot::Star => ClipboardKind::Primary,
        _ => ClipboardKind::Clipboard,
    }
}

fn clipboard_register(text: String) -> Register {
    match text.strip_suffix('\n') {
        Some(text) => Register::from_parts(text.to_string(), RegisterKind::LineWise),
        None => Register::from_parts(text, RegisterKind::CharWise),
    }
}

fn clipboard_text(register: &Register) -> String {
    match register.kind() {
        RegisterKind::LineWise => format!("{}\n", register.text()),
        RegisterKind::CharWise | RegisterKind::BlockWise => register.text().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn quoted_count_multiplies_next_count() {
        let mut e = editor_with("one\ntwo");
        e.apply(Action::YankLine);
        e.apply(Action::PushCount(2));
        e.apply(Action::SelectRegister('0'));
        e.apply(Action::PushCount(3));
        e.apply(Action::PasteAfter);
        assert_eq!(
            e.text_lines(),
            vec!["one", "one", "one", "one", "one", "one", "one", "two"]
        );
    }

    #[test]
    fn named_yank_pastes_via_register() {
        let mut e = editor_with("one\ntwo");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::YankLine);
        e.apply(Action::MoveDown);
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["one", "two", "one"]);
    }

    #[test]
    fn append_register_concatenates_lines() {
        let mut e = editor_with("one\ntwo\nthree");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::YankLine);
        e.apply(Action::MoveDown);
        e.apply(Action::SelectRegister('A'));
        e.apply(Action::YankLine);
        e.apply(Action::GotoLastLine);
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["one", "two", "three", "one", "two"]);
    }

    #[test]
    fn small_delete_uses_dash_register() {
        let mut e = editor_with("abc");
        e.apply(Action::DeleteForward);
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::SelectRegister('-'));
        e.apply(Action::PasteBefore);
        assert_eq!(e.text_lines(), vec!["bac"]);
    }

    #[test]
    fn numbered_delete_shifts_registers() {
        let mut e = editor_with("one\ntwo\nthree");
        e.apply(Action::DeleteLine);
        e.apply(Action::DeleteLine);
        e.apply(Action::GotoLastLine);
        e.apply(Action::SelectRegister('2'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["three", "one"]);
    }

    #[test]
    fn black_hole_delete_keeps_unnamed() {
        let mut e = editor_with("one\ntwo");
        e.apply(Action::YankLine);
        e.apply(Action::SelectRegister('_'));
        e.apply(Action::DeleteLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["two", "one"]);
    }

    #[test]
    fn insert_register_inserts_linewise() {
        let mut e = editor_with("one\ntwo");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::YankLine);
        e.apply(Action::GotoLastLine);
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertRegister('a'));
        e.apply(Action::ExitInsert);
        assert_eq!(e.text_lines(), vec!["one", "twoone", ""]);
        assert_eq!(e.register_text(), "one");
    }

    #[test]
    fn repeat_keeps_register() {
        let mut e = editor_with("one\ntwo\nthree");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::DeleteLine);
        e.apply(Action::RepeatLastChange);
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["three", "two"]);
    }

    #[test]
    fn invalid_register_select_is_ignored() {
        let mut e = editor_with("one");
        e.apply(Action::SelectRegister('?'));
        e.apply(Action::YankLine);
        assert_eq!(e.register_text(), "one");
    }

    #[derive(Default)]
    struct ClipboardState {
        text: Option<String>,
        written: Vec<String>,
    }

    struct FakeClipboard(std::rc::Rc<std::cell::RefCell<ClipboardState>>);

    impl crate::clipboard::Clipboard for FakeClipboard {
        fn read(&mut self, _kind: ClipboardKind) -> Option<String> {
            self.0.borrow().text.clone()
        }

        fn write(&mut self, _kind: ClipboardKind, text: &str) {
            self.0.borrow_mut().written.push(text.to_string());
        }
    }

    fn editor_with_clipboard(
        text: &str,
    ) -> (Editor, std::rc::Rc<std::cell::RefCell<ClipboardState>>) {
        let state = std::rc::Rc::new(std::cell::RefCell::new(ClipboardState::default()));
        let mut e = editor_with(text);
        e.set_clipboard(Box::new(FakeClipboard(state.clone())));
        (e, state)
    }

    #[test]
    fn unnamed_yank_writes_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::YankLine);
        assert_eq!(state.borrow().written, vec!["one\n".to_string()]);
    }

    #[test]
    fn small_delete_writes_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::DeleteForward);
        assert_eq!(state.borrow().written, vec!["o".to_string()]);
    }

    #[test]
    fn named_yank_does_not_write_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::YankLine);
        assert!(state.borrow().written.is_empty());
    }

    #[test]
    fn unnamed_put_reads_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        state.borrow_mut().text = Some("external text".to_string());
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["oexternal textne", "two"]);
    }

    #[test]
    fn unnamed_put_refreshes_external_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::YankLine);
        state.borrow_mut().text = Some("replaced\n".to_string());
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["one", "replaced", "two"]);
    }

    #[test]
    fn clipboard_yank_writes_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::SelectRegister('+'));
        e.apply(Action::YankLine);
        assert_eq!(state.borrow().written, vec!["one\n".to_string()]);
    }

    #[test]
    fn clipboard_put_reads_system_clipboard() {
        let (mut e, state) = editor_with_clipboard("one\ntwo");
        state.borrow_mut().text = Some("three\n".to_string());
        e.apply(Action::SelectRegister('+'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["one", "three", "two"]);
    }

    #[test]
    fn clipboard_put_without_content_uses_unnamed() {
        let (mut e, _state) = editor_with_clipboard("one\ntwo");
        e.apply(Action::YankLine);
        e.apply(Action::SelectRegister('+'));
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines(), vec!["one", "one", "two"]);
    }

    #[test]
    fn visual_paste_over_selection_keeps_register() {
        let mut e = editor_with("one\ntwo\nthree");
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::YankLine);
        e.apply(Action::MoveDown);
        e.apply(Action::EnterVisualLine);
        e.apply(Action::SelectRegister('a'));
        e.apply(Action::PasteOverSelection);
        assert_eq!(e.text_lines(), vec!["one", "one", "three"]);
        assert_eq!(
            e.registers
                .get(RegisterSlot::Named(0))
                .map(|register| register.text().to_string()),
            Some("one".to_string())
        );
    }
}
