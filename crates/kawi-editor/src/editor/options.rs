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
pub struct IndentOptions {
    pub shiftwidth: Option<usize>,
    pub expandtab: Option<bool>,
    pub tabstop: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageIndent {
    pub defaults: IndentOptions,
    pub overrides: IndentOptions,
    pub language: Option<LanguageId>,
    pub rules: IndentRules,
}

impl LanguageIndent {
    pub fn tabstop(&self) -> Option<usize> {
        self.overrides.tabstop.or(self.defaults.tabstop)
    }
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

use super::indent::IndentRules;
use super::Editor;
use crate::editing::LanguageEditing;
use kawi_syntax::LanguageId;

impl Editor {
    pub fn set_buffer_defaults(&mut self, defaults: BufferOptionDefaults) {
        self.buffer_defaults = defaults;
        self.refresh_folds();
    }

    pub fn buffer_defaults(&self) -> BufferOptionDefaults {
        self.buffer_defaults
    }

    pub fn set_language_indent(&mut self, indent: LanguageIndent) {
        if self.language_indent != indent {
            self.language_indent = indent;
            self.bump_language_generation();
            self.refresh_folds();
        }
    }

    pub fn set_language_editing(&mut self, editing: LanguageEditing) {
        if self.language_editing != editing {
            self.language_editing = editing;
            self.bump_language_generation();
        }
    }

    pub fn language_generation(&self) -> u64 {
        self.language_generation
    }

    fn bump_language_generation(&mut self) {
        self.language_generation = self.language_generation.wrapping_add(1);
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

    pub fn set_match_brackets(&mut self, enabled: bool) {
        self.match_brackets = enabled;
    }

    pub fn match_brackets(&self) -> bool {
        self.match_brackets
    }

    pub(super) fn reset_buffer_options(&mut self) {
        self.runtime_shiftwidth = None;
        self.runtime_expandtab = None;
        self.set_language_indent(LanguageIndent::default());
        self.set_language_editing(LanguageEditing::default());
        self.autoindent = self.buffer_defaults.autoindent;
        self.refresh_folds();
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

    fn indent(shiftwidth: usize, expandtab: bool, tabstop: usize) -> IndentOptions {
        IndentOptions {
            shiftwidth: Some(shiftwidth),
            expandtab: Some(expandtab),
            tabstop: Some(tabstop),
        }
    }

    fn defaults(shiftwidth: usize, expandtab: bool) -> BufferOptionDefaults {
        BufferOptionDefaults {
            shiftwidth: Some(shiftwidth),
            expandtab: Some(expandtab),
            autoindent: true,
        }
    }

    #[test]
    fn language_defaults_precede_built_in_indent() {
        let mut editor = editor();
        editor.set_language_indent(LanguageIndent {
            defaults: indent(2, true, 2),
            ..LanguageIndent::default()
        });
        assert_eq!(editor.shiftwidth(), 2);
        assert!(editor.expandtab());
        assert_eq!(editor.tabstop(), 2);
    }

    #[test]
    fn config_defaults_precede_language_defaults() {
        let mut editor = editor();
        editor.set_buffer_defaults(defaults(6, false));
        editor.set_language_indent(LanguageIndent {
            defaults: indent(2, true, 2),
            ..LanguageIndent::default()
        });
        assert_eq!(editor.shiftwidth(), 6);
        assert!(!editor.expandtab());
        assert_eq!(editor.tabstop(), 2);
    }

    #[test]
    fn language_overrides_precede_config_defaults() {
        let mut editor = editor();
        editor.set_buffer_defaults(defaults(6, false));
        editor.set_language_indent(LanguageIndent {
            defaults: indent(4, true, 4),
            overrides: indent(2, true, 3),
            ..LanguageIndent::default()
        });
        assert_eq!(editor.shiftwidth(), 2);
        assert!(editor.expandtab());
        assert_eq!(editor.tabstop(), 3);
    }

    #[test]
    fn runtime_indent_precedes_language_overrides() {
        let mut editor = editor();
        editor.set_buffer_defaults(defaults(6, false));
        editor.set_language_indent(LanguageIndent {
            defaults: indent(4, true, 4),
            overrides: indent(2, true, 3),
            ..LanguageIndent::default()
        });
        editor.set_shiftwidth(5);
        editor.set_expandtab(false);
        assert_eq!(editor.shiftwidth(), 5);
        assert!(!editor.expandtab());
        assert_eq!(editor.tabstop(), 3);
    }

    #[test]
    fn language_overrides_can_set_one_field() {
        let mut editor = editor();
        editor.set_language_indent(LanguageIndent {
            defaults: indent(4, true, 4),
            overrides: IndentOptions {
                shiftwidth: Some(2),
                ..IndentOptions::default()
            },
            ..LanguageIndent::default()
        });
        assert_eq!(editor.shiftwidth(), 2);
        assert!(editor.expandtab());
        assert_eq!(editor.tabstop(), 4);
    }

    #[test]
    fn built_in_indent_is_the_last_fallback() {
        let editor = editor();
        assert_eq!(editor.shiftwidth(), 8);
        assert!(!editor.expandtab());
        assert_eq!(editor.tabstop(), 8);
    }
}
