#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferOptionDefaults {
    pub shiftwidth: Option<usize>,
    pub expandtab: Option<bool>,
    pub autoindent: bool,
}

impl Default for BufferOptionDefaults {
    fn default() -> Self {
        Self {
            shiftwidth: None,
            expandtab: None,
            autoindent: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LanguageIndent {
    pub shiftwidth: Option<usize>,
    pub expandtab: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowOptions {
    pub number: bool,
    pub relativenumber: bool,
    pub numberwidth: usize,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            number: true,
            relativenumber: false,
            numberwidth: 4,
        }
    }
}

use super::Editor;
use crate::editing::LanguageEditing;

impl Editor {
    pub fn set_buffer_defaults(&mut self, defaults: BufferOptionDefaults) {
        self.buffer_defaults = defaults;
    }

    pub fn buffer_defaults(&self) -> BufferOptionDefaults {
        self.buffer_defaults
    }

    pub fn set_language_indent(&mut self, indent: LanguageIndent) {
        if self.language_indent != indent {
            self.language_indent = indent;
        }
    }

    pub fn set_language_editing(&mut self, editing: LanguageEditing) {
        if self.language_editing != editing {
            self.language_editing = editing;
        }
    }

    pub fn language_editing(&self) -> &LanguageEditing {
        &self.language_editing
    }

    pub fn set_auto_pairs(&mut self, enabled: bool) {
        self.auto_pairs = enabled;
    }

    pub fn auto_pairs(&self) -> bool {
        self.auto_pairs
    }

    pub(super) fn reset_buffer_options(&mut self) {
        self.runtime_shiftwidth = None;
        self.runtime_expandtab = None;
        self.language_indent = LanguageIndent::default();
        self.language_editing = LanguageEditing::default();
        self.autoindent = self.buffer_defaults.autoindent;
    }

    pub fn shiftwidth(&self) -> usize {
        self.effective_indent().shiftwidth
    }

    pub fn window_options(&self) -> WindowOptions {
        WindowOptions {
            number: self.number,
            relativenumber: self.relativenumber,
            numberwidth: self.numberwidth,
        }
    }

    pub fn set_window_options(&mut self, options: WindowOptions) {
        self.number = options.number;
        self.relativenumber = options.relativenumber;
        self.numberwidth = options.numberwidth;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn editor() -> Editor {
        Editor::new(Buffer::from_string("text"))
    }

    #[test]
    fn language_indent_precedes_config_defaults() {
        let mut editor = editor();
        editor.set_buffer_defaults(BufferOptionDefaults {
            shiftwidth: Some(6),
            expandtab: Some(false),
            autoindent: true,
        });
        editor.set_language_indent(LanguageIndent {
            shiftwidth: Some(2),
            expandtab: Some(true),
        });
        assert_eq!(editor.shiftwidth(), 2);
        assert!(editor.expandtab());
    }

    #[test]
    fn runtime_indent_precedes_language_settings() {
        let mut editor = editor();
        editor.set_buffer_defaults(BufferOptionDefaults {
            shiftwidth: Some(6),
            expandtab: Some(false),
            autoindent: true,
        });
        editor.set_language_indent(LanguageIndent {
            shiftwidth: Some(2),
            expandtab: Some(true),
        });
        editor.set_shiftwidth(5);
        editor.set_expandtab(false);
        assert_eq!(editor.shiftwidth(), 5);
        assert!(!editor.expandtab());
    }

    #[test]
    fn built_in_indent_is_the_last_fallback() {
        let editor = editor();
        assert_eq!(editor.shiftwidth(), 8);
        assert!(!editor.expandtab());
    }
}
