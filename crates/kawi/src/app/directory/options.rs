use super::metadata::MetadataColumns;
use super::sort::{SortKey, SortOptions};
use crate::app::App;
use crate::options::OptionValue;

const DIRTY_OPTION_MESSAGE: &str =
    "Directory option changed; save or undo changes before reloading";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct DirectoryOptions {
    pub(in crate::app) show_hidden: bool,
    pub(in crate::app) confirm: bool,
    pub(in crate::app) trash: bool,
    pub(in crate::app) sort: SortOptions,
    pub(in crate::app) metadata: MetadataColumns,
}

impl Default for DirectoryOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            confirm: true,
            trash: true,
            sort: SortOptions::default(),
            metadata: MetadataColumns::all(),
        }
    }
}

impl DirectoryOptions {
    pub(in crate::app) fn listing_changed_from(self, previous: Self) -> bool {
        self.show_hidden != previous.show_hidden
            || self.sort != previous.sort
            || self.metadata != previous.metadata
    }
}

impl App {
    pub(in crate::app) fn apply_directory_option(
        &mut self,
        name: &str,
        value: &OptionValue,
    ) -> Option<bool> {
        let previous = self.directory_options;
        match name {
            "directory_show_hidden" => {
                self.directory_options.show_hidden = value.as_bool().unwrap_or(false);
            }
            "directory_confirm" => {
                self.directory_options.confirm = value.as_bool().unwrap_or(true);
            }
            "directory_trash" => {
                self.directory_options.trash = value.as_bool().unwrap_or(true);
            }
            "directory_sort" => {
                self.directory_options.sort.key =
                    value.as_text().and_then(SortKey::parse).unwrap_or_default();
            }
            "directory_sort_reverse" => {
                self.directory_options.sort.reverse = value.as_bool().unwrap_or(false);
            }
            "directory_sort_directories_first" => {
                self.directory_options.sort.directories_first = value.as_bool().unwrap_or(true);
            }
            "directory_metadata" => {
                self.directory_options.metadata = value
                    .as_text()
                    .and_then(MetadataColumns::parse)
                    .unwrap_or_default();
            }
            _ => return None,
        }
        Some(self.directory_options.listing_changed_from(previous))
    }

    pub(in crate::app) fn reload_directory_listings(&mut self) {
        let numbers: Vec<usize> = self.directory.numbers().collect();
        let mut needs_reload = false;
        let mut error = None;
        for number in numbers {
            self.mark_directory_view_pending(number);
            if self.editor.buffer_is_dirty(number).unwrap_or(false) {
                needs_reload = true;
                continue;
            }
            if self.directory_loading.is_loading(number) {
                self.directory_loading.forget(number);
            }
            let focus = self.directory_focus_text(number);
            match self.start_directory_reload(number) {
                Ok(()) => {
                    if let Some(text) = focus {
                        self.directory_loading.defer_focus(number, text);
                    }
                }
                Err(message) => {
                    self.editor.set_buffer_readonly(number, false);
                    error.get_or_insert(message);
                }
            }
        }
        if let Some(message) = error {
            self.show_message(message);
        } else if needs_reload {
            self.show_message(DIRTY_OPTION_MESSAGE.to_string());
        }
    }

    fn mark_directory_view_pending(&mut self, number: usize) {
        if let Some(state) = self.directory.get_mut(number) {
            state.view_options_pending = true;
        }
    }

    pub(in crate::app) fn apply_pending_directory_view(&mut self, number: usize) {
        let options = self.directory_options;
        let Some(state) = self.directory.get_mut(number) else {
            return;
        };
        if !state.view_options_pending {
            return;
        }
        state.show_hidden = options.show_hidden;
        state.sort = options.sort;
        state.view_options_pending = false;
    }
}
