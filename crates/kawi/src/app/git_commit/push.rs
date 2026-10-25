use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent};

use super::TaskKind;
use crate::app::App;
use kawi_task::ParserSpec;

pub(super) struct PendingPush {
    pub branch: String,
    pub args: Vec<String>,
}

impl App {
    pub(crate) fn start_git_push(&mut self, force: bool) -> anyhow::Result<bool> {
        let Some(status) = self.git_status.status().cloned() else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        if status.is_detached() {
            self.show_message("Cannot push a detached HEAD".to_string());
            return Ok(false);
        }
        let branch = status.branch_label().to_string();
        let args = push_args(&status, force);
        if status.upstream().is_none() {
            self.show_message("No upstream; pushing with -u origin HEAD".to_string());
        }
        if force {
            self.git_commit.pending_push = Some(PendingPush { branch, args });
            let branch = self
                .git_commit
                .pending_push
                .as_ref()
                .map(|pending| pending.branch.clone())
                .unwrap_or_default();
            self.show_message(format!("Force push {branch}? y/n"));
            return Ok(false);
        }
        self.start_push_task(args, status.root);
        Ok(false)
    }

    pub(crate) fn handle_git_push_confirmation(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let Some(pending) = self.git_commit.pending_push.take() else {
            return Ok(false);
        };
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                let Some(root) = self.git_status.status().map(|status| status.root.clone()) else {
                    return Ok(false);
                };
                self.start_push_task(pending.args, root);
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.show_message(format!("Force push {} cancelled", pending.branch));
            }
            _ => {
                self.git_commit.pending_push = Some(pending);
            }
        }
        Ok(false)
    }

    fn start_push_task(&mut self, args: Vec<String>, root: std::path::PathBuf) {
        let mut env = BTreeMap::new();
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        self.start_git_task(super::GitTask {
            name: "git push".to_string(),
            args,
            env,
            detached: true,
            kind: TaskKind::Push,
            parser: ParserSpec::None,
            root,
        });
    }
}

fn push_args(status: &kawi_git::Status, force: bool) -> Vec<String> {
    let mut args = vec!["push".to_string()];
    if force {
        args.push("--force-with-lease".to_string());
    }
    if status.upstream().is_none() {
        args.extend(["-u".to_string(), "origin".to_string(), "HEAD".to_string()]);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_git::{BranchStatus, Status, Upstream};

    fn status(upstream: Option<Upstream>) -> Status {
        Status {
            root: Default::default(),
            git_dir: Default::default(),
            branch: BranchStatus::Named {
                name: "main".to_string(),
                oid: Some("abc".to_string()),
                upstream,
            },
            operation: kawi_git::OperationState::None,
            entries: Vec::new(),
        }
    }

    #[test]
    fn push_without_upstream_sets_it() {
        assert_eq!(
            push_args(&status(None), false),
            vec!["push", "-u", "origin", "HEAD"]
        );
    }

    #[test]
    fn force_push_uses_force_with_lease() {
        let upstream = Upstream {
            name: "origin/main".to_string(),
            ahead: 1,
            behind: 0,
        };
        assert_eq!(
            push_args(&status(Some(upstream)), true),
            vec!["push", "--force-with-lease"]
        );
    }

    #[test]
    fn force_push_without_upstream_keeps_both_options() {
        assert_eq!(
            push_args(&status(None), true),
            vec!["push", "--force-with-lease", "-u", "origin", "HEAD"]
        );
    }
}
