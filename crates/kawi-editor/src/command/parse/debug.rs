use crate::command::{Command, DebugCommand, ParseFailure};

use super::invalid_argument;

pub(super) fn parse(words: &[&str]) -> Result<Command, ParseFailure> {
    let command = match words {
        [] | ["status" | "info"] => DebugCommand::Status,
        ["start"] => DebugCommand::Start(None),
        ["start", name] => DebugCommand::Start(Some((*name).to_string())),
        ["restart"] => DebugCommand::Restart,
        ["stop"] => DebugCommand::Stop,
        ["terminate"] => DebugCommand::Terminate,
        ["panel"] => DebugCommand::Panel,
        ["continue" | "run"] => DebugCommand::Continue,
        ["next" | "over"] => DebugCommand::Next,
        ["step" | "in" | "into"] => DebugCommand::StepIn,
        ["out"] => DebugCommand::StepOut,
        ["breakpoint" | "toggle"] => DebugCommand::ToggleBreakpoint,
        ["breakpoint", "condition", rest @ ..] if !rest.is_empty() => {
            DebugCommand::BreakpointCondition(rest.join(" "))
        }
        ["breakpoint", "hit" | "hit-condition", rest @ ..] if !rest.is_empty() => {
            DebugCommand::BreakpointHitCondition(rest.join(" "))
        }
        ["breakpoint", "log" | "log-message", rest @ ..] if !rest.is_empty() => {
            DebugCommand::BreakpointLogMessage(rest.join(" "))
        }
        ["breakpoint", "clear"] => DebugCommand::BreakpointClear,
        ["breakpoints"] => DebugCommand::Breakpoints,
        ["function", rest @ ..] if !rest.is_empty() => {
            DebugCommand::FunctionBreakpoint(rest.join(" "))
        }
        ["functions"] => DebugCommand::FunctionBreakpoints,
        ["exceptions"] => DebugCommand::Exceptions,
        ["exception", filter] => DebugCommand::ToggleException((*filter).to_string()),
        ["stack"] => DebugCommand::Stack,
        ["variables"] => DebugCommand::Variables,
        ["expand", reference] => match reference.parse() {
            Ok(reference) if reference > 0 => DebugCommand::ExpandVariables(reference),
            _ => return Err(invalid_argument(words)),
        },
        ["more", reference] => match reference.parse() {
            Ok(reference) if reference > 0 => DebugCommand::MoreVariables(reference),
            _ => return Err(invalid_argument(words)),
        },
        ["set", reference, name, rest @ ..] if !rest.is_empty() => {
            let Ok(reference) = reference.parse() else {
                return Err(invalid_argument(words));
            };
            if reference <= 0 {
                return Err(invalid_argument(words));
            }
            let value = rest
                .strip_prefix(&["="])
                .map(|value| value.join(" "))
                .unwrap_or_else(|| rest.join(" "));
            DebugCommand::SetVariable {
                reference,
                name: (*name).to_string(),
                value,
            }
        }
        ["frame", frame_id] => match frame_id.parse() {
            Ok(frame_id) if frame_id > 0 => DebugCommand::Frame(frame_id),
            _ => return Err(invalid_argument(words)),
        },
        ["threads"] => DebugCommand::Threads,
        ["thread", thread_id] => match thread_id.parse() {
            Ok(thread_id) if thread_id > 0 => DebugCommand::Thread(thread_id),
            _ => return Err(invalid_argument(words)),
        },
        ["eval" | "evaluate", rest @ ..] if !rest.is_empty() => {
            DebugCommand::Evaluate(rest.join(" "))
        }
        ["console"] => DebugCommand::Console,
        ["watch", "add", rest @ ..] if !rest.is_empty() => DebugCommand::Watch(rest.join(" ")),
        ["watch", "edit", index, rest @ ..] if !rest.is_empty() => match index.parse() {
            Ok(index) if index > 0 => DebugCommand::EditWatch(index, rest.join(" ")),
            _ => return Err(invalid_argument(words)),
        },
        ["watch", "remove" | "delete", rest @ ..] if !rest.is_empty() => {
            DebugCommand::Unwatch(rest.join(" "))
        }
        ["watch"] | ["watches"] => DebugCommand::Watches,
        ["unwatch", rest @ ..] if !rest.is_empty() => DebugCommand::Unwatch(rest.join(" ")),
        ["output"] => DebugCommand::Output,
        ["configurations" | "configs"] => DebugCommand::Configurations,
        _ => return Err(invalid_argument(words)),
    };
    Ok(Command::Debug(command))
}
