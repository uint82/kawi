use kawi_syntax::{file_size_supported, LanguageId, SyntaxKey, SyntaxRequest};

use super::Editor;

impl Editor {
    pub fn language_id(&self) -> Option<LanguageId> {
        LanguageId::from_path(self.buffer.path.as_deref()?)
    }

    pub fn syntax_key(&self) -> Option<SyntaxKey> {
        let language = self.language_id()?;
        if !file_size_supported(self.buffer.len_bytes()) {
            return None;
        }
        let top = self.view_top();
        let end = self
            .visible_rows(top, self.view_height())
            .last()
            .map(|row| row.saturating_add(1))
            .unwrap_or(top)
            .min(self.buffer.line_count());
        if top >= end {
            return None;
        }
        Some(SyntaxKey {
            document: self.buffer_number() as u64,
            revision: self.buffer.revision(),
            top,
            end,
            language,
        })
    }

    pub fn syntax_request(&self) -> Option<SyntaxRequest> {
        let key = self.syntax_key()?;
        Some(SyntaxRequest {
            key,
            text: self.buffer.snapshot(),
        })
    }

    pub fn buffer_revision(&self) -> u64 {
        self.buffer.revision()
    }
}

#[cfg(test)]
mod tests {
    use kawi_buffer::Buffer;
    use kawi_syntax::LanguageId;

    use super::*;

    fn editor(path: &str, text: &str) -> Editor {
        let mut buffer = Buffer::from_string(text);
        buffer.path = Some(path.to_string());
        let mut editor = Editor::new(buffer);
        editor.set_view_size(20, 2);
        editor
    }

    #[test]
    fn builds_a_request_for_a_supported_file() {
        let editor = editor("test.rs", "fn main() {}\nfn other() {}\nfn third() {}\n");
        let request = editor.syntax_request().expect("request");
        assert_eq!(request.key.document, 1);
        assert_eq!(request.key.revision, 0);
        assert_eq!(request.key.top, 0);
        assert_eq!(request.key.end, 2);
        assert_eq!(request.key.language, LanguageId::Rust);
        assert_eq!(
            request.text.to_string(),
            "fn main() {}\nfn other() {}\nfn third() {}\n"
        );
    }

    #[test]
    fn skips_unknown_file_types() {
        let editor = editor("test.txt", "plain\n");
        assert!(editor.syntax_key().is_none());
        assert!(editor.syntax_request().is_none());
    }

    #[test]
    fn skips_files_over_the_size_limit() {
        let text = "a".repeat(kawi_syntax::MAX_FILE_BYTES.saturating_add(1));
        let editor = editor("big.rs", &text);
        assert!(editor.syntax_key().is_none());
        assert!(editor.syntax_request().is_none());
    }

    #[test]
    fn skips_unnamed_buffers() {
        let editor = Editor::new(Buffer::from_string("fn main() {}\n"));
        assert!(editor.syntax_key().is_none());
    }
}
