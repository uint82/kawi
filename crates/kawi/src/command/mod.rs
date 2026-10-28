use std::collections::HashMap;

use kawi_editor::Action;

mod builtin;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CommandId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandHandler {
    Action(Action),
    SendKeys,
    Noop,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandSpec {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: String,
    pub(crate) handler: CommandHandler,
}

pub(crate) struct CommandRegistry {
    specs: Vec<CommandSpec>,
    index: HashMap<String, CommandId>,
}

impl CommandRegistry {
    pub(crate) fn new() -> Self {
        let mut specs = Vec::new();
        let mut index = HashMap::new();
        for builtin in builtin::all() {
            let id = CommandId(specs.len() as u32);
            insert_name(&mut index, builtin.name, id);
            for alias in builtin.aliases {
                insert_name(&mut index, alias, id);
            }
            specs.push(CommandSpec {
                name: builtin.name.to_string(),
                aliases: builtin
                    .aliases
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                description: builtin.description.to_string(),
                handler: builtin.handler,
            });
        }
        Self { specs, index }
    }

    pub(crate) fn find(&self, name: &str) -> Option<CommandId> {
        self.index.get(name.trim()).copied()
    }

    pub(crate) fn spec(&self, id: CommandId) -> Option<&CommandSpec> {
        self.specs.get(id.0 as usize)
    }

    pub(crate) fn specs(&self) -> impl Iterator<Item = &CommandSpec> {
        self.specs.iter()
    }
}

fn insert_name(index: &mut HashMap<String, CommandId>, name: &str, id: CommandId) {
    assert!(
        index.insert(name.to_string(), id).is_none(),
        "duplicate command name: {name}"
    );
}
