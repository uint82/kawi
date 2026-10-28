use kawi_editor::Action;

use super::CommandHandler;

mod core;
mod navigation;
mod tools;

pub(crate) struct Builtin {
    pub(crate) name: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) description: &'static str,
    pub(crate) handler: CommandHandler,
}

pub(crate) fn all() -> impl Iterator<Item = &'static Builtin> {
    core::COMMANDS
        .iter()
        .chain(navigation::COMMANDS)
        .chain(tools::COMMANDS)
}

const fn action(name: &'static str, action: Action, description: &'static str) -> Builtin {
    Builtin {
        name,
        aliases: &[],
        description,
        handler: CommandHandler::Action(action),
    }
}

const fn aliased(
    name: &'static str,
    aliases: &'static [&'static str],
    action: Action,
    description: &'static str,
) -> Builtin {
    Builtin {
        name,
        aliases,
        description,
        handler: CommandHandler::Action(action),
    }
}
