use crate::command::{Command, GitCommand, ParseFailure};

use super::invalid_argument;

pub(super) fn parse(words: &[&str]) -> Result<Command, ParseFailure> {
    let command = match words {
        [] => GitCommand::Diff,
        ["status"] => GitCommand::Status,
        ["commit"] => GitCommand::Commit { all: false },
        ["commit", "-a" | "all"] => GitCommand::Commit { all: true },
        ["amend"] => GitCommand::Amend { staged: false },
        ["amend", "staged"] => GitCommand::Amend { staged: true },
        ["push"] => GitCommand::Push { force: false },
        ["push", "--force-with-lease" | "force"] => GitCommand::Push { force: true },
        ["fetch"] => GitCommand::Fetch,
        ["branches"] | ["branch"] => GitCommand::Branches,
        ["branch", "-d", name] => GitCommand::DeleteBranch {
            name: (*name).to_string(),
            force: false,
        },
        ["branch", "-D", name] => GitCommand::DeleteBranch {
            name: (*name).to_string(),
            force: true,
        },
        ["branch", name] => GitCommand::CreateBranch {
            name: (*name).to_string(),
            base: None,
        },
        ["branch", name, base] => GitCommand::CreateBranch {
            name: (*name).to_string(),
            base: Some((*base).to_string()),
        },
        ["switch" | "checkout", target] => GitCommand::Switch {
            target: (*target).to_string(),
        },
        ["log", rest @ ..] => parse_git_log(rest),
        ["next_hunk"] => GitCommand::NextHunk,
        ["prev_hunk"] => GitCommand::PrevHunk,
        ["stage_hunk"] => GitCommand::StageHunk,
        ["undo_stage_hunk"] => GitCommand::UndoStage,
        ["stage_buffer"] => GitCommand::StageBuffer,
        ["preview_hunk"] => GitCommand::PreviewHunk,
        ["diff"] => GitCommand::Diff,
        ["refresh"] => GitCommand::Refresh,
        ["next_conflict"] => GitCommand::NextConflict,
        ["prev_conflict"] => GitCommand::PrevConflict,
        ["ours"] => GitCommand::ConflictOurs,
        ["theirs"] => GitCommand::ConflictTheirs,
        ["both"] => GitCommand::ConflictBoth,
        ["continue"] => GitCommand::Continue,
        ["abort"] => GitCommand::Abort,
        _ => return Err(invalid_argument(words)),
    };
    Ok(Command::Git(command))
}

fn parse_git_log(words: &[&str]) -> GitCommand {
    let mut file = false;
    let mut grep = None;
    let mut author = None;
    let mut index = 0;
    while index < words.len() {
        match words[index] {
            "%" => {
                file = true;
                index += 1;
            }
            "--grep" => {
                grep = Some(words[index + 1..].join(" "));
                break;
            }
            "--author" => {
                author = Some(words[index + 1..].join(" "));
                break;
            }
            _ => index += 1,
        }
    }
    GitCommand::Log { file, grep, author }
}
