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

impl Editor {
    pub fn set_buffer_defaults(&mut self, defaults: BufferOptionDefaults) {
        self.buffer_defaults = defaults;
    }

    pub fn buffer_defaults(&self) -> BufferOptionDefaults {
        self.buffer_defaults
    }

    pub(super) fn reset_buffer_options(&mut self) {
        self.shiftwidth_override = self.buffer_defaults.shiftwidth;
        self.expandtab_override = self.buffer_defaults.expandtab;
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
