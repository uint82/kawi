use kawi_buffer::UndoOffset;

use super::diagnostic;
use super::{
    file_ops, set, substitute, undo, Command, ExplorerCommand, LspCommand, ParseFailure,
    PickerCommand, QuickfixCommand, TaskCommand, UndoCommand,
};

mod debug;
mod git;

pub fn parse(text: &str) -> Result<Command, ParseFailure> {
    if text.trim().is_empty() {
        return Err(ParseFailure::EmptyInput);
    }
    if let Some(result) = parse_named(text) {
        return result;
    }
    let name = command_name(text).unwrap_or_default();
    if name.starts_with("se") {
        return Err(ParseFailure::unknown_command(name));
    }
    if let Some(result) = substitute::parse(text) {
        return result.map(Command::Substitute);
    }
    Err(ParseFailure::unknown_command(name))
}

fn parse_named(text: &str) -> Option<Result<Command, ParseFailure>> {
    if let Some(result) = file_ops::parse(text) {
        return Some(result);
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let (command, rest) = words.split_first()?;
    let name = *command;
    let result = match (name, rest) {
        ("q" | "quit", []) => Ok(Command::Quit),
        ("wq" | "writequit" | "wq!" | "writequit!", []) => write_quit_command(name),
        ("q!" | "quit!", []) => Ok(Command::ForceQuit),
        ("qa" | "quitall", []) => Ok(Command::QuitAll),
        ("qa!" | "quitall!", []) => Ok(Command::ForceQuitAll),
        ("wall", []) => Ok(Command::WriteAll),
        ("wqa" | "wqall", []) => Ok(Command::WriteQuitAll),
        ("ls" | "buffers" | "files", []) => Ok(Command::ListBuffers),
        ("marks", []) => Ok(Command::Marks),
        ("messages", []) => Ok(Command::Messages { clear: false }),
        ("messages", ["clear"]) => Ok(Command::Messages { clear: true }),
        ("bn" | "bnext", []) => Ok(Command::NextBuffer),
        ("bp" | "bprevious", []) => Ok(Command::PreviousBuffer),
        ("b#", []) => Ok(Command::AlternateBuffer),
        ("e" | "edit" | "e!" | "edit!", rest) if rest.len() <= 1 => {
            edit_command(name, rest.first().copied())
        }
        ("w" | "write" | "w!" | "write!", rest) if rest.len() <= 1 => {
            write_command(name, rest.first().copied())
        }
        ("bd" | "bdelete" | "bd!" | "bdelete!", rest) if rest.len() <= 1 => {
            delete_command(name, rest.first().copied())
        }
        ("b" | "buffer", ["#"]) => Ok(Command::AlternateBuffer),
        ("b" | "buffer", [arg]) => Ok(Command::SelectBuffer(arg.to_string())),
        ("sp" | "split", rest) if rest.len() <= 1 => Ok(Command::Split {
            vertical: false,
            name: rest.first().map(|name| (*name).to_string()),
        }),
        ("vs" | "vsplit", rest) if rest.len() <= 1 => Ok(Command::Split {
            vertical: true,
            name: rest.first().map(|name| (*name).to_string()),
        }),
        ("new", rest) if rest.len() <= 1 => Ok(Command::NewWindow {
            vertical: false,
            name: rest.first().map(|name| (*name).to_string()),
        }),
        ("vnew", rest) if rest.len() <= 1 => Ok(Command::NewWindow {
            vertical: true,
            name: rest.first().map(|name| (*name).to_string()),
        }),
        ("cq" | "cquit", []) => Ok(Command::Cquit),
        ("clo" | "close", []) => Ok(Command::CloseWindow),
        ("on" | "only", []) => Ok(Command::OnlyWindow),
        ("pick", []) => Ok(Command::Pick(PickerCommand::Files)),
        ("pick", ["file" | "files"]) => Ok(Command::Pick(PickerCommand::Files)),
        ("pick", ["grep" | "live_grep" | "grep_live"]) => Ok(Command::Pick(PickerCommand::Grep)),
        ("pick", ["buffer" | "buffers"]) => Ok(Command::Pick(PickerCommand::Buffers)),
        ("pick", ["replace"]) => Ok(Command::Pick(PickerCommand::Replace)),
        ("pick", ["symbol" | "symbols"]) => Ok(Command::Pick(PickerCommand::Symbols)),
        ("pick", ["workspace_symbol" | "workspace_symbols" | "workspace-symbols"]) => {
            Ok(Command::Pick(PickerCommand::WorkspaceSymbols))
        }
        ("replace", []) => Ok(Command::Pick(PickerCommand::Replace)),
        ("git", rest) => git::parse(rest),
        ("task", rest) => parse_task_command(rest),
        ("quickfix" | "qf", rest) => parse_quickfix_command(rest),
        ("lsp", rest) => parse_lsp_command(rest),
        ("debug", rest) => debug::parse(rest),
        ("explorer", []) => Ok(Command::Explorer(ExplorerCommand::Toggle)),
        ("explorer", ["focus"]) => Ok(Command::Explorer(ExplorerCommand::Focus)),
        ("explorer", ["reveal"]) => Ok(Command::Explorer(ExplorerCommand::Reveal)),
        ("term" | "terminal", rest) => Ok(Command::Terminal {
            command: terminal_command(rest),
            vertical: false,
        }),
        ("vterm" | "vterminal", rest) => Ok(Command::Terminal {
            command: terminal_command(rest),
            vertical: true,
        }),
        ("conflict", []) => Ok(Command::Conflict),
        ("format", []) => Ok(Command::Format),
        ("h" | "help", rest) => Ok(Command::Help {
            topic: (!rest.is_empty()).then(|| rest.join(" ")),
        }),
        ("health" | "checkhealth" | "doctor", rest) => Ok(Command::Health {
            topic: (!rest.is_empty()).then(|| rest.join(" ")),
        }),
        ("u" | "undo", []) => Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(1)))),
        ("u" | "undo", [arg]) => undo::jump_command(arg),
        ("earlier", []) => Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(1)))),
        ("earlier", [arg]) => undo::offset_command(arg, true),
        ("later", []) => Ok(Command::Undo(UndoCommand::Later(UndoOffset::States(1)))),
        ("later", [arg]) => undo::offset_command(arg, false),
        ("undolist", []) => Ok(Command::Undo(UndoCommand::List)),
        ("undolist", [arg]) => Err(ParseFailure::trailing_characters(*arg)),
        ("copen", []) => Ok(Command::Quickfix(QuickfixCommand::Open)),
        ("cclose", []) => Ok(Command::Quickfix(QuickfixCommand::Close)),
        ("cnext", []) => Ok(Command::Quickfix(QuickfixCommand::Next)),
        ("cprev", []) => Ok(Command::Quickfix(QuickfixCommand::Previous)),
        ("set", rest) => set::parse(rest),
        ("nmap" | "imap" | "vmap" | "nunmap" | "iunmap" | "vunmap" | "map" | "commands", _) => {
            Ok(Command::Map {
                text: text.to_string(),
            })
        }
        ("plugin", rest) => Ok(Command::Plugin {
            text: rest.join(" "),
        }),
        _ if substitute::is_substitute_name(name) => return None,
        _ => return known_command_failure(name, rest),
    };
    Some(result)
}

fn command_name(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    Some(&trimmed[..end])
}

fn known_command_failure(name: &str, rest: &[&str]) -> Option<Result<Command, ParseFailure>> {
    if !diagnostic::is_known_command(name) {
        return None;
    }
    Some(if rest.is_empty() {
        Err(ParseFailure::missing_argument(name))
    } else {
        Err(ParseFailure::invalid_argument(rest.join(" ")))
    })
}

fn write_command(command: &str, name: Option<&str>) -> Result<Command, ParseFailure> {
    let (force, base) = split_bang(command);
    if !matches!(base, "w" | "write") {
        return Err(ParseFailure::invalid_argument(command));
    }
    Ok(match name {
        Some(name) => Command::WriteAs {
            name: name.to_string(),
            force,
        },
        None => Command::Write { force },
    })
}

fn write_quit_command(command: &str) -> Result<Command, ParseFailure> {
    let (force, base) = split_bang(command);
    if !matches!(base, "wq" | "writequit") {
        return Err(ParseFailure::invalid_argument(command));
    }
    Ok(Command::WriteQuit { force })
}

fn delete_command(command: &str, spec: Option<&str>) -> Result<Command, ParseFailure> {
    let (force, base) = split_bang(command);
    if !matches!(base, "bd" | "bdelete") {
        return Err(ParseFailure::invalid_argument(command));
    }
    Ok(Command::DeleteBuffer {
        spec: spec.map(str::to_string),
        force,
    })
}

fn split_bang(command: &str) -> (bool, &str) {
    match command.strip_suffix('!') {
        Some(command) => (true, command),
        None => (false, command),
    }
}

fn edit_command(command: &str, name: Option<&str>) -> Result<Command, ParseFailure> {
    let (force, base) = split_bang(command);
    if !matches!(base, "e" | "edit") {
        return Err(ParseFailure::invalid_argument(command));
    }
    Ok(Command::Edit {
        name: name.map(str::to_string),
        force,
    })
}

fn parse_task_command(words: &[&str]) -> Result<Command, ParseFailure> {
    let command = match words {
        [] | ["list"] => TaskCommand::List,
        ["rerun"] => TaskCommand::Rerun,
        ["cancel"] => TaskCommand::Cancel,
        ["output"] => TaskCommand::Output,
        ["close"] => TaskCommand::Close,
        ["run"] => return Err(invalid_argument(words)),
        ["run", name] => TaskCommand::Run((*name).to_string()),
        [name] => TaskCommand::Run((*name).to_string()),
        _ => return Err(invalid_argument(words)),
    };
    Ok(Command::Task(command))
}

fn terminal_command(words: &[&str]) -> Option<String> {
    (!words.is_empty()).then(|| words.join(" "))
}

fn parse_quickfix_command(words: &[&str]) -> Result<Command, ParseFailure> {
    let command = match words {
        [] | ["open"] => QuickfixCommand::Open,
        ["next"] => QuickfixCommand::Next,
        ["prev" | "previous"] => QuickfixCommand::Previous,
        ["close"] => QuickfixCommand::Close,
        _ => return Err(invalid_argument(words)),
    };
    Ok(Command::Quickfix(command))
}

fn parse_lsp_command(words: &[&str]) -> Result<Command, ParseFailure> {
    match words {
        [] | ["status" | "info"] => Ok(Command::Lsp(LspCommand::Status)),
        ["start"] => Ok(Command::Lsp(LspCommand::Start(None))),
        ["start", server] => Ok(Command::Lsp(LspCommand::Start(Some((*server).to_string())))),
        ["stop"] => Ok(Command::Lsp(LspCommand::Stop(None))),
        ["stop", server] => Ok(Command::Lsp(LspCommand::Stop(Some((*server).to_string())))),
        ["restart"] => Ok(Command::Lsp(LspCommand::Restart(None))),
        ["restart", server] => Ok(Command::Lsp(LspCommand::Restart(Some(
            (*server).to_string(),
        )))),
        _ => Err(invalid_argument(words)),
    }
}

pub(super) fn invalid_argument(words: &[&str]) -> ParseFailure {
    ParseFailure::invalid_argument(words.join(" "))
}
