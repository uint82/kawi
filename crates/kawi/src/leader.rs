use kawi_editor::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaderPath {
    Root,
    Group(&'static LeaderGroup),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LeaderGroup {
    pub(crate) key: char,
    pub(crate) label: &'static str,
    pub(crate) visual: bool,
    pub(crate) commands: &'static [LeaderCommand],
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LeaderCommand {
    pub(crate) key: char,
    pub(crate) aliases: &'static [char],
    pub(crate) label: &'static str,
    pub(crate) action: Action,
}

const FIND_COMMANDS: &[LeaderCommand] = &[
    LeaderCommand {
        key: 'f',
        aliases: &['p'],
        label: "find project file",
        action: Action::OpenFilePicker,
    },
    LeaderCommand {
        key: 'k',
        aliases: &[],
        label: "find buffer",
        action: Action::OpenBufferPicker,
    },
    LeaderCommand {
        key: 'r',
        aliases: &[],
        label: "find and replace",
        action: Action::OpenReplacePicker,
    },
    LeaderCommand {
        key: 's',
        aliases: &[],
        label: "find in project",
        action: Action::OpenGrepPicker,
    },
    LeaderCommand {
        key: 'w',
        aliases: &[],
        label: "find current word",
        action: Action::OpenGrepWordPicker,
    },
];

const CODE_COMMANDS: &[LeaderCommand] = &[
    LeaderCommand {
        key: 'a',
        aliases: &[],
        label: "code action",
        action: Action::CodeAction,
    },
    LeaderCommand {
        key: 'f',
        aliases: &[],
        label: "format code",
        action: Action::Format,
    },
];

const SYMBOL_COMMANDS: &[LeaderCommand] = &[
    LeaderCommand {
        key: 's',
        aliases: &[],
        label: "document symbols",
        action: Action::OpenDocumentSymbols,
    },
    LeaderCommand {
        key: 'w',
        aliases: &[],
        label: "workspace symbols",
        action: Action::OpenWorkspaceSymbols,
    },
];

const RENAME_COMMANDS: &[LeaderCommand] = &[LeaderCommand {
    key: 'n',
    aliases: &[],
    label: "rename symbol",
    action: Action::Rename,
}];

const TASK_COMMANDS: &[LeaderCommand] = &[
    LeaderCommand {
        key: 'c',
        aliases: &[],
        label: "cancel task",
        action: Action::TaskCancel,
    },
    LeaderCommand {
        key: 'l',
        aliases: &[],
        label: "list tasks",
        action: Action::TaskList,
    },
    LeaderCommand {
        key: 'o',
        aliases: &[],
        label: "task output",
        action: Action::TaskOutput,
    },
    LeaderCommand {
        key: 'q',
        aliases: &[],
        label: "open quickfix",
        action: Action::QuickfixOpen,
    },
    LeaderCommand {
        key: 'r',
        aliases: &[],
        label: "rerun task",
        action: Action::TaskRerun,
    },
    LeaderCommand {
        key: 't',
        aliases: &[],
        label: "toggle terminal",
        action: Action::ToggleTerminal,
    },
];

const GIT_COMMANDS: &[LeaderCommand] = &[
    LeaderCommand {
        key: 'd',
        aliases: &[],
        label: "show diff",
        action: Action::GitDiff,
    },
    LeaderCommand {
        key: 'p',
        aliases: &[],
        label: "preview hunk",
        action: Action::GitPreviewHunk,
    },
    LeaderCommand {
        key: 's',
        aliases: &[],
        label: "stage hunk",
        action: Action::GitStageHunk,
    },
    LeaderCommand {
        key: 'S',
        aliases: &[],
        label: "stage buffer",
        action: Action::GitStageBuffer,
    },
    LeaderCommand {
        key: 'u',
        aliases: &[],
        label: "undo stage",
        action: Action::GitUndoStage,
    },
];

pub(crate) const LEADER_GROUPS: &[LeaderGroup] = &[
    LeaderGroup {
        key: 'c',
        label: "code",
        visual: true,
        commands: CODE_COMMANDS,
    },
    LeaderGroup {
        key: 'f',
        label: "find",
        visual: true,
        commands: FIND_COMMANDS,
    },
    LeaderGroup {
        key: 'r',
        label: "rename",
        visual: false,
        commands: RENAME_COMMANDS,
    },
    LeaderGroup {
        key: 's',
        label: "symbols",
        visual: false,
        commands: SYMBOL_COMMANDS,
    },
    LeaderGroup {
        key: 't',
        label: "tasks",
        visual: false,
        commands: TASK_COMMANDS,
    },
    LeaderGroup {
        key: 'v',
        label: "git",
        visual: false,
        commands: GIT_COMMANDS,
    },
];

pub(crate) fn group(key: char) -> Option<&'static LeaderGroup> {
    LEADER_GROUPS.iter().find(|group| group.key == key)
}

pub(crate) fn groups(visual: bool) -> impl Iterator<Item = &'static LeaderGroup> {
    LEADER_GROUPS
        .iter()
        .filter(move |group| !visual || group.visual)
}

pub(crate) fn action(path: LeaderPath, key: char) -> Option<Action> {
    let LeaderPath::Group(group) = path else {
        return None;
    };
    group
        .commands
        .iter()
        .find(|command| command.key == key || command.aliases.contains(&key))
        .map(|command| command.action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn groups_cover_the_leader_prefixes() {
        let keys: Vec<char> = LEADER_GROUPS.iter().map(|group| group.key).collect();
        assert_eq!(keys, ['c', 'f', 'r', 's', 't', 'v']);
    }

    #[test]
    fn visual_groups_are_limited_to_supported_actions() {
        let keys: Vec<char> = groups(true).map(|group| group.key).collect();
        assert_eq!(keys, ['c', 'f']);
    }

    #[test]
    fn project_file_alias_resolves() {
        let path = LeaderPath::Group(group('f').expect("find group"));
        assert_eq!(action(path, 'p'), Some(Action::OpenFilePicker));
    }

    #[test]
    fn symbols_use_the_s_group() {
        let path = LeaderPath::Group(group('s').expect("symbols group"));
        assert_eq!(action(path, 's'), Some(Action::OpenDocumentSymbols));
        assert_eq!(action(path, 'w'), Some(Action::OpenWorkspaceSymbols));
    }

    #[test]
    fn unknown_commands_do_not_resolve() {
        let path = LeaderPath::Group(group('f').expect("find group"));
        assert_eq!(action(path, 'x'), None);
        assert_eq!(action(LeaderPath::Root, 'f'), None);
    }

    #[test]
    fn every_command_and_alias_resolves() {
        for group in LEADER_GROUPS {
            let path = LeaderPath::Group(group);
            for command in group.commands {
                assert_eq!(action(path, command.key), Some(command.action));
                for alias in command.aliases {
                    assert_eq!(action(path, *alias), Some(command.action));
                }
            }
        }
    }

    #[test]
    fn group_keys_and_aliases_are_unique() {
        for group in LEADER_GROUPS {
            let mut keys = HashSet::new();
            for command in group.commands {
                assert!(
                    keys.insert(command.key),
                    "duplicate leader key {} in group {}",
                    command.key,
                    group.key
                );
                for alias in command.aliases {
                    assert!(
                        keys.insert(*alias),
                        "duplicate leader alias {} in group {}",
                        alias,
                        group.key
                    );
                }
            }
        }
    }
}
