pub(super) const UNKNOWN_TASK: &str = "K001";
pub(super) const NO_PREVIOUS_TASK: &str = "K002";
pub(super) const NO_RUNNING_TASK: &str = "K003";
pub(super) const NO_TASK_OUTPUT: &str = "K004";
pub(super) const TASK_ALREADY_RUNNING: &str = "K005";
pub(super) const NO_GIT_EDITOR: &str = "K006";
pub(super) const LSP_TARGET_REQUIRED: &str = "K007";
pub(super) const LSP_SERVICE_STOPPED: &str = "K008";
pub(super) const NO_KEYMAP_FILE: &str = "K100";
pub(super) const NOT_A_DIRECTORY: &str = "K101";
pub(super) const UNKNOWN_THEME: &str = "K102";
pub(super) const UNKNOWN_KEYMAP: &str = "K103";
pub(super) const INVALID_THEME: &str = "K104";

#[cfg(test)]
pub(super) const ALL: &[&str] = &[
    UNKNOWN_TASK,
    NO_PREVIOUS_TASK,
    NO_RUNNING_TASK,
    NO_TASK_OUTPUT,
    TASK_ALREADY_RUNNING,
    NO_GIT_EDITOR,
    LSP_TARGET_REQUIRED,
    LSP_SERVICE_STOPPED,
    NO_KEYMAP_FILE,
    NOT_A_DIRECTORY,
    UNKNOWN_THEME,
    UNKNOWN_KEYMAP,
    INVALID_THEME,
];
