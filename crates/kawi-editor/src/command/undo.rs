use kawi_buffer::UndoOffset;

use super::{Command, ParseFailure};

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

pub(super) fn jump_command(arg: &str) -> Result<Command, ParseFailure> {
    match arg.parse::<u64>() {
        Ok(sequence) => Ok(Command::Undo(UndoCommand::Jump(sequence))),
        Err(_) => Err(ParseFailure::invalid_argument(arg)),
    }
}

pub(super) fn offset_command(arg: &str, earlier: bool) -> Result<Command, ParseFailure> {
    match parse_offset(arg) {
        Ok(offset) => {
            let command = if earlier {
                UndoCommand::Earlier(offset)
            } else {
                UndoCommand::Later(offset)
            };
            Ok(Command::Undo(command))
        }
        Err(()) => Err(ParseFailure::invalid_argument(arg)),
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
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(1))))
        );
        assert_eq!(
            parse("u"),
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(1))))
        );
        assert_eq!(parse("undo 12"), Ok(Command::Undo(UndoCommand::Jump(12))));
        assert_eq!(
            parse("earlier"),
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(1))))
        );
        assert_eq!(
            parse("earlier 3"),
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::States(3))))
        );
        assert_eq!(
            parse("earlier 10m"),
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::Seconds(
                600
            ))))
        );
        assert_eq!(
            parse("earlier 1f"),
            Ok(Command::Undo(UndoCommand::Earlier(UndoOffset::FileWrites(
                1
            ))))
        );
        assert_eq!(
            parse("later 2h"),
            Ok(Command::Undo(UndoCommand::Later(UndoOffset::Seconds(7200))))
        );
        assert_eq!(parse("undolist"), Ok(Command::Undo(UndoCommand::List)));
        assert_eq!(
            parse("earlier nope"),
            Err(ParseFailure::invalid_argument("nope"))
        );
        assert_eq!(
            parse("undo nope"),
            Err(ParseFailure::invalid_argument("nope"))
        );
        assert_eq!(
            parse("undolist extra"),
            Err(ParseFailure::trailing_characters("extra"))
        );
    }
}
