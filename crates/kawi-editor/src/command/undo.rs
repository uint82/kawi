use kawi_buffer::UndoOffset;

use super::Command;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoCommand {
    Earlier(UndoOffset),
    Later(UndoOffset),
    Jump(u64),
    List,
}

pub(super) fn jump_command(arg: &str) -> Command {
    match arg.parse::<u64>() {
        Ok(sequence) => Command::Undo(UndoCommand::Jump(sequence)),
        Err(_) => Command::SetError(format!("E474: Invalid argument: {arg}")),
    }
}

pub(super) fn offset_command(arg: &str, earlier: bool) -> Command {
    match parse_offset(arg) {
        Ok(offset) => {
            let command = if earlier {
                UndoCommand::Earlier(offset)
            } else {
                UndoCommand::Later(offset)
            };
            Command::Undo(command)
        }
        Err(()) => Command::SetError(format!("E474: Invalid argument: {arg}")),
    }
}

fn parse_offset(arg: &str) -> Result<UndoOffset, ()> {
    let split = arg
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(arg.len());
    let (number, unit) = arg.split_at(split);
    let count = number.parse::<usize>().map_err(|_| ())?;
    match unit {
        "" => Ok(UndoOffset::States(count)),
        "f" => Ok(UndoOffset::FileWrites(count)),
        "s" => Ok(UndoOffset::Seconds(count as u64)),
        "m" => Ok(UndoOffset::Seconds(
            (count as u64).saturating_mul(SECONDS_PER_MINUTE),
        )),
        "h" => Ok(UndoOffset::Seconds(
            (count as u64).saturating_mul(SECONDS_PER_HOUR),
        )),
        "d" => Ok(UndoOffset::Seconds(
            (count as u64).saturating_mul(SECONDS_PER_DAY),
        )),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn parses_undo_commands() {
        assert_eq!(
            parse("undo"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::States(1)))
        );
        assert_eq!(
            parse("u"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::States(1)))
        );
        assert_eq!(parse("undo 12"), Command::Undo(UndoCommand::Jump(12)));
        assert_eq!(
            parse("earlier"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::States(1)))
        );
        assert_eq!(
            parse("earlier 3"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::States(3)))
        );
        assert_eq!(
            parse("earlier 10m"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::Seconds(600)))
        );
        assert_eq!(
            parse("earlier 1f"),
            Command::Undo(UndoCommand::Earlier(UndoOffset::FileWrites(1)))
        );
        assert_eq!(
            parse("later 2h"),
            Command::Undo(UndoCommand::Later(UndoOffset::Seconds(7200)))
        );
        assert_eq!(parse("undolist"), Command::Undo(UndoCommand::List));
        assert_eq!(
            parse("earlier nope"),
            Command::SetError("E474: Invalid argument: nope".to_string())
        );
        assert_eq!(
            parse("undo nope"),
            Command::SetError("E474: Invalid argument: nope".to_string())
        );
        assert_eq!(
            parse("undolist extra"),
            Command::SetError("E488: Trailing characters: extra".to_string())
        );
    }
}
