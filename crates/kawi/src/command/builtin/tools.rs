use kawi_editor::Action;

use super::{action, Builtin};

pub(super) const COMMANDS: &[Builtin] = &[
    action("visual.exit", Action::ExitVisual, "leave visual mode"),
    action("visual.cancel", Action::CancelVisual, "cancel visual mode"),
    action(
        "visual.reselect",
        Action::ReselectVisual,
        "reselect the last visual range",
    ),
    action(
        "visual.swap_ends",
        Action::SwapSelectionEnds,
        "swap the selection ends",
    ),
    action(
        "visual.collapse",
        Action::CollapseSelection,
        "collapse the selection",
    ),
    action(
        "visual.inner_word",
        Action::SelectInnerWord,
        "select the inner word",
    ),
    action(
        "visual.around_word",
        Action::SelectAroundWord,
        "select around the word",
    ),
    action(
        "picker.files",
        Action::OpenFilePicker,
        "open the file picker",
    ),
    action(
        "picker.buffers",
        Action::OpenBufferPicker,
        "open the buffer picker",
    ),
    action("picker.grep", Action::OpenGrepPicker, "open live grep"),
    action(
        "picker.grep_word",
        Action::OpenGrepWordPicker,
        "grep the word under the cursor",
    ),
    action(
        "picker.replace",
        Action::OpenReplacePicker,
        "open the replace picker",
    ),
    action(
        "picker.symbols",
        Action::OpenDocumentSymbols,
        "open document symbols",
    ),
    action(
        "picker.workspace_symbols",
        Action::OpenWorkspaceSymbols,
        "open workspace symbols",
    ),
    action(
        "buffer.alternate",
        Action::EditAlternate,
        "open the alternate buffer",
    ),
    action(
        "lsp.definition",
        Action::GotoDefinition,
        "go to the definition",
    ),
    action(
        "lsp.references",
        Action::ShowReferences,
        "show the references",
    ),
    action("lsp.hover", Action::Hover, "show the hover information"),
    action("lsp.rename", Action::Rename, "rename the symbol"),
    action(
        "lsp.code_action",
        Action::CodeAction,
        "show the code actions",
    ),
    action("lsp.format", Action::Format, "format the buffer"),
    action(
        "diagnostic.next",
        Action::DiagnosticNext,
        "move to the next diagnostic",
    ),
    action(
        "diagnostic.previous",
        Action::DiagnosticPrev,
        "move to the previous diagnostic",
    ),
    action(
        "diagnostic.show",
        Action::ShowDiagnostic,
        "show the diagnostic",
    ),
    action(
        "quickfix.next",
        Action::QuickfixNext,
        "move to the next quickfix entry",
    ),
    action(
        "quickfix.previous",
        Action::QuickfixPrev,
        "move to the previous quickfix entry",
    ),
    action(
        "quickfix.open",
        Action::QuickfixOpen,
        "open the quickfix list",
    ),
    action(
        "git.next_hunk",
        Action::GitNextHunk,
        "move to the next hunk",
    ),
    action(
        "git.previous_hunk",
        Action::GitPrevHunk,
        "move to the previous hunk",
    ),
    action("git.stage_hunk", Action::GitStageHunk, "stage the hunk"),
    action("git.undo_stage", Action::GitUndoStage, "unstage the hunk"),
    action(
        "git.stage_buffer",
        Action::GitStageBuffer,
        "stage the buffer",
    ),
    action(
        "git.preview_hunk",
        Action::GitPreviewHunk,
        "preview the hunk",
    ),
    action("git.diff", Action::GitDiff, "show the buffer diff"),
    action("task.list", Action::TaskList, "list the tasks"),
    action("task.rerun", Action::TaskRerun, "rerun the last task"),
    action("task.cancel", Action::TaskCancel, "cancel the task"),
    action("task.output", Action::TaskOutput, "show the task output"),
    action(
        "terminal.toggle",
        Action::ToggleTerminal,
        "toggle the terminal",
    ),
    action(
        "explorer.toggle",
        Action::ToggleExplorer,
        "toggle the explorer",
    ),
    action(
        "explorer.focus",
        Action::FocusExplorer,
        "focus the explorer",
    ),
    action(
        "explorer.reveal",
        Action::RevealInExplorer,
        "reveal the current file",
    ),
    action(
        "explorer.parent",
        Action::OpenParentDirectory,
        "open the parent directory",
    ),
];
