use std::collections::HashMap;

use kawi_editor::Action;

mod builtin;
#[cfg(test)]
mod tests;

pub(crate) use builtin::BUILTINS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CommandId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandSource {
    Builtin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandHandler {
    Action(Action),
    SendKeys,
    Noop,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandSpec {
    pub(crate) id: CommandId,
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: String,
    pub(crate) handler: CommandHandler,
    pub(crate) source: CommandSource,
}

pub(crate) struct CommandRegistry {
    specs: Vec<CommandSpec>,
    index: HashMap<String, CommandId>,
}

impl CommandRegistry {
    pub(crate) fn new() -> Self {
        let mut specs = Vec::with_capacity(BUILTINS.len());
        let mut index = HashMap::new();
        for builtin in BUILTINS {
            let id = CommandId(specs.len() as u32);
            insert_name(&mut index, builtin.name, id);
            for alias in builtin.aliases {
                insert_name(&mut index, alias, id);
            }
            specs.push(CommandSpec {
                id,
                name: builtin.name.to_string(),
                aliases: builtin
                    .aliases
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                description: builtin.description.to_string(),
                handler: builtin.handler,
                source: CommandSource::Builtin,
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

    pub(crate) fn names(&self) -> Vec<&str> {
        self.specs
            .iter()
            .map(|spec| spec.name.as_str())
            .chain(
                self.specs
                    .iter()
                    .flat_map(|spec| spec.aliases.iter().map(String::as_str)),
            )
            .collect()
    }
}

fn insert_name(index: &mut HashMap<String, CommandId>, name: &str, id: CommandId) {
    if index.insert(name.to_string(), id).is_none() {
        return;
    }
    index.remove(name);
    panic!("duplicate command name: {name}");
}
