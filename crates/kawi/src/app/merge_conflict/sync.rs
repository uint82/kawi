use std::borrow::Cow;

use kawi_git::conflict::{self, ConflictBlock};
use ropey::{Rope, RopeSlice};

use super::MergeConflictState;

struct Line<'a>(Cow<'a, str>);

impl AsRef<str> for Line<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

fn rope_line(line: RopeSlice<'_>) -> Line<'_> {
    match line.as_str() {
        Some(text) => Line(Cow::Borrowed(text)),
        None => Line(Cow::Owned(line.to_string())),
    }
}

impl MergeConflictState {
    pub(super) fn sync_rope(
        &mut self,
        buffer_id: Option<String>,
        revision: u64,
        rope: &Rope,
    ) -> bool {
        let blocks = conflict::parse(rope.lines().map(rope_line));
        self.replace_blocks(buffer_id, revision, blocks)
    }

    fn replace_blocks(
        &mut self,
        buffer_id: Option<String>,
        revision: u64,
        blocks: Vec<ConflictBlock>,
    ) -> bool {
        let changed = blocks != self.blocks;
        self.blocks = blocks;
        self.buffer_id = buffer_id;
        self.revision = revision;
        self.synced = true;
        changed
    }
}
