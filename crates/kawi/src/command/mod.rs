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
    Plugin(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandSource {
    Builtin,
    Plugin(String),
}

impl CommandSource {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Builtin => String::new(),
            Self::Plugin(name) => format!(" [plugin: {name}]"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CommandSpec {
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

    pub(crate) fn register_plugin(
        &mut self,
        name: String,
        description: String,
        plugin: String,
        target: u32,
    ) -> Result<CommandId, String> {
        if self.index.contains_key(&name) {
            return Err(format!("command name already exists: {name}"));
        }
        let id = CommandId(self.specs.len() as u32);
        self.index.insert(name.clone(), id);
        self.specs.push(CommandSpec {
            name,
            aliases: Vec::new(),
            description,
            handler: CommandHandler::Plugin(target),
            source: CommandSource::Plugin(plugin),
        });
        Ok(id)
    }
}

fn insert_name(index: &mut HashMap<String, CommandId>, name: &str, id: CommandId) {
    assert!(
        index.insert(name.to_string(), id).is_none(),
        "duplicate command name: {name}"
    );
}
