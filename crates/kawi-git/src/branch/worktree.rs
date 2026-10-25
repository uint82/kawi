use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::command::{run_read, stderr};

use super::parser::parse_worktree_list;

pub(super) fn worktree_paths(root: &Path) -> Result<HashMap<String, PathBuf>> {
    let output = run_read(root, &["worktree", "list", "--porcelain"])?;
    if !output.status.success() {
        anyhow::bail!("git worktree list failed: {}", stderr(&output));
    }
    Ok(parse_worktree_list(&output.stdout))
}
