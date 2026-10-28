use super::listing::SortMode;
use super::metadata::MetadataColumns;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct DirectoryOptions {
    pub(in crate::app) show_hidden: bool,
    pub(in crate::app) confirm: bool,
    pub(in crate::app) trash: bool,
    pub(in crate::app) sort: SortMode,
    pub(in crate::app) metadata: MetadataColumns,
}

impl Default for DirectoryOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            confirm: true,
            trash: true,
            sort: SortMode::DirectoriesFirst,
            metadata: MetadataColumns::all(),
        }
    }
}
