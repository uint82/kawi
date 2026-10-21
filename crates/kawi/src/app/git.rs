use kawi_git::{
    apply_hunks, apply_index_patch, diff_hunks, ensure_intent_to_add, filter_common, hunk_at,
    next_hunk, patch_hunks, prev_hunk, reset_index, GitFile, Hunk, PatchSettings, SignKind,
};
use kawi_tui::frame::GitSign;

pub struct GitNav {
    pub row: usize,
    pub index: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStage {
    Staged,
    Unstaged,
}

pub struct GitSession {
    file: GitFile,
    base: Vec<String>,
    hunks: Vec<Hunk>,
    staged_hunks: Vec<Hunk>,
    signs: Vec<Option<GitSign>>,
    revision: u64,
    buffer_no_final_newline: bool,
    undo: Option<UndoStage>,
}

struct UndoStage {
    base: Vec<String>,
    hunks: Vec<Hunk>,
    was_untracked: bool,
    no_final_newline: bool,
}

impl GitSession {
    pub fn new(
        file: GitFile,
        lines: &[String],
        revision: u64,
        buffer_no_final_newline: bool,
    ) -> Self {
        let base = file.base.clone();
        let mut session = Self {
            file,
            base,
            hunks: Vec::new(),
            staged_hunks: Vec::new(),
            signs: Vec::new(),
            revision: revision.wrapping_sub(1),
            buffer_no_final_newline,
            undo: None,
        };
        session.rebuild(lines, revision);
        session
    }

    pub fn update(
        &mut self,
        lines: &[String],
        revision: u64,
        buffer_no_final_newline: bool,
    ) -> bool {
        if self.revision == revision && self.buffer_no_final_newline == buffer_no_final_newline {
            return false;
        }
        self.buffer_no_final_newline = buffer_no_final_newline;
        self.rebuild(lines, revision);
        true
    }

    pub fn buffer_no_final_newline(&self) -> bool {
        self.buffer_no_final_newline
    }

    pub fn visible_signs(&self, top: usize, height: usize) -> Vec<Option<GitSign>> {
        (top..top.saturating_add(height))
            .map(|row| self.signs.get(row).copied().flatten())
            .collect()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn hunk_at(&self, row: usize) -> Option<usize> {
        hunk_at(&self.hunks, row.saturating_add(1))
    }

    pub fn next(&self, row: usize) -> Option<GitNav> {
        self.nav(row, true)
    }

    pub fn prev(&self, row: usize) -> Option<GitNav> {
        self.nav(row, false)
    }

    pub fn stage_hunk(
        &mut self,
        row: usize,
        lines: &[String],
        revision: u64,
    ) -> Result<GitStage, String> {
        let line = row.saturating_add(1);
        if let Some(index) = hunk_at(&self.hunks, line) {
            let hunk = self.hunks[index].clone();
            self.stage_hunks(vec![hunk], lines, revision)?;
            return Ok(GitStage::Staged);
        }
        let Some(index) = hunk_at(&self.staged_hunks, line) else {
            return Err("No hunk to stage".to_string());
        };
        let hunk = self.staged_hunks[index].clone();
        self.unstage_hunks(vec![hunk], lines, revision)?;
        Ok(GitStage::Unstaged)
    }

    pub fn stage_buffer(&mut self, lines: &[String], revision: u64) -> Result<(), String> {
        if self.hunks.is_empty() {
            return Err("No unstaged changes in file to stage".to_string());
        }
        self.stage_hunks(self.hunks.clone(), lines, revision)
    }

    pub fn undo_stage(&mut self, lines: &[String], revision: u64) -> Result<(), String> {
        let Some(undo) = self.undo.take() else {
            return Err("No staged hunk to undo".to_string());
        };
        let result = if undo.was_untracked {
            reset_index(&self.file.root, &self.file.relpath).map_err(|error| error.to_string())
        } else {
            let reversed = reversed_hunks(&undo.hunks);
            self.apply_reversed(&reversed, &undo.base, undo.no_final_newline)
        };
        if let Err(error) = result {
            self.undo = Some(undo);
            return Err(error);
        }
        self.base = undo.base;
        self.file.untracked = undo.was_untracked;
        self.file.no_final_newline = undo.no_final_newline;
        self.rebuild(lines, revision);
        Ok(())
    }

    pub fn preview_hunk(&self, row: usize) -> Result<Vec<String>, String> {
        let Some(index) = self.hunk_at(row) else {
            return Err("No hunk to preview".to_string());
        };
        Ok(hunk_preview(&self.hunks[index]))
    }

    pub fn diff_lines(&self) -> Vec<String> {
        let buffer = apply_hunks(&self.base, &self.hunks);
        let settings = self.patch_settings(&buffer, self.buffer_no_final_newline);
        let patch = patch_hunks(&self.file.relpath, &self.file.mode, settings, &self.hunks);
        patch.lines().map(str::to_string).collect()
    }

    fn patch_settings<'a>(
        &'a self,
        buffer: &'a [String],
        buffer_no_final_newline: bool,
    ) -> PatchSettings<'a> {
        PatchSettings {
            base: &self.base,
            buffer,
            crlf: self.file.crlf,
            base_no_final_newline: self.file.no_final_newline,
            buffer_no_final_newline,
        }
    }

    fn nav(&self, row: usize, forward: bool) -> Option<GitNav> {
        let line = row.saturating_add(1);
        let index = if forward {
            next_hunk(&self.hunks, line)?
        } else {
            prev_hunk(&self.hunks, line)?
        };
        let hunk = &self.hunks[index];
        let line_count = self.signs.len().max(1);
        let target = if forward {
            hunk.start_line()
        } else {
            hunk.end_line()
        };
        Some(GitNav {
            row: target.clamp(1, line_count).saturating_sub(1),
            index: index.saturating_add(1),
            total: self.hunks.len(),
        })
    }

    fn rebuild(&mut self, lines: &[String], revision: u64) {
        let lines = normalize_lines(lines);
        self.hunks = diff_hunks(&self.base, &lines);
        let head_hunks = diff_hunks(&self.file.head, &lines);
        self.staged_hunks = filter_common(&head_hunks, &self.hunks);
        self.signs = sign_map(
            &self.hunks,
            &self.staged_hunks,
            lines.len(),
            self.file.untracked,
        );
        self.revision = revision;
    }

    fn stage_hunks(
        &mut self,
        hunks: Vec<Hunk>,
        lines: &[String],
        revision: u64,
    ) -> Result<(), String> {
        let was_untracked = self.file.untracked;
        if was_untracked {
            ensure_intent_to_add(&self.file).map_err(|error| error.to_string())?;
        }
        let buffer = normalize_lines(lines);
        let settings = self.patch_settings(&buffer, self.buffer_no_final_newline);
        let patch = patch_hunks(&self.file.relpath, &self.file.mode, settings, &hunks);
        apply_index_patch(&self.file.root, &patch).map_err(|error| error.to_string())?;
        let previous = self.base.clone();
        let no_final_newline = self.file.no_final_newline;
        let touches_eof = hunks
            .iter()
            .any(|hunk| hunk_touches_eof(hunk, self.base.len()));
        self.base = apply_hunks(&self.base, &hunks);
        self.file.untracked = false;
        if touches_eof {
            self.file.no_final_newline = self.buffer_no_final_newline;
        }
        self.undo = Some(UndoStage {
            base: previous,
            hunks,
            was_untracked,
            no_final_newline,
        });
        self.rebuild(lines, revision);
        Ok(())
    }

    fn unstage_hunks(
        &mut self,
        hunks: Vec<Hunk>,
        lines: &[String],
        revision: u64,
    ) -> Result<(), String> {
        let reversed = reversed_hunks(&hunks);
        self.apply_reversed(&reversed, &self.file.head, self.file.head_no_final_newline)?;
        let touches_eof = reversed
            .iter()
            .any(|hunk| hunk_touches_eof(hunk, self.base.len()));
        self.base = apply_hunks(&self.base, &reversed);
        self.file.untracked = false;
        if touches_eof {
            self.file.no_final_newline = self.file.head_no_final_newline;
        }
        self.undo = None;
        self.rebuild(lines, revision);
        Ok(())
    }

    fn apply_reversed(
        &self,
        reversed: &[Hunk],
        buffer: &[String],
        buffer_no_final_newline: bool,
    ) -> Result<(), String> {
        let settings = self.patch_settings(buffer, buffer_no_final_newline);
        let patch = patch_hunks(&self.file.relpath, &self.file.mode, settings, reversed);
        apply_index_patch(&self.file.root, &patch).map_err(|error| error.to_string())
    }
}

fn reversed_hunks(hunks: &[Hunk]) -> Vec<Hunk> {
    hunks.iter().map(Hunk::reversed).collect()
}

fn hunk_touches_eof(hunk: &Hunk, base_len: usize) -> bool {
    if hunk.removed_count == 0 {
        hunk.removed_start >= base_len
    } else {
        hunk.removed_start.saturating_add(hunk.removed_count) > base_len
    }
}

fn normalize_lines(lines: &[String]) -> Vec<String> {
    if lines.len() == 1 && lines[0].is_empty() {
        Vec::new()
    } else {
        lines.to_vec()
    }
}

fn sign_map(
    hunks: &[Hunk],
    staged_hunks: &[Hunk],
    line_count: usize,
    untracked: bool,
) -> Vec<Option<GitSign>> {
    let mut signs = vec![None; line_count];
    apply_signs(&mut signs, hunks, untracked, false);
    apply_signs(&mut signs, staged_hunks, false, true);
    signs
}

fn apply_signs(signs: &mut [Option<GitSign>], hunks: &[Hunk], untracked: bool, staged: bool) {
    for hunk in hunks {
        for sign in hunk.signs() {
            if sign.row >= signs.len() || (staged && signs[sign.row].is_some()) {
                continue;
            }
            signs[sign.row] = Some(git_sign(sign.kind, untracked, staged));
        }
    }
}

fn git_sign(kind: SignKind, untracked: bool, staged: bool) -> GitSign {
    match (kind, untracked, staged) {
        (SignKind::Add, true, false) => GitSign::Untracked,
        (SignKind::Add, _, false) => GitSign::Add,
        (SignKind::Change, _, false) => GitSign::Change,
        (SignKind::Delete, _, false) => GitSign::Delete,
        (SignKind::TopDelete, _, false) => GitSign::TopDelete,
        (SignKind::ChangeDelete, _, false) => GitSign::ChangeDelete,
        (SignKind::Add, _, true) => GitSign::StagedAdd,
        (SignKind::Change, _, true) => GitSign::StagedChange,
        (SignKind::Delete, _, true) => GitSign::StagedDelete,
        (SignKind::TopDelete, _, true) => GitSign::StagedTopDelete,
        (SignKind::ChangeDelete, _, true) => GitSign::StagedChangeDelete,
    }
}

fn hunk_preview(hunk: &Hunk) -> Vec<String> {
    let mut lines = vec![format!(
        "@@ -{},{} +{},{} @@",
        hunk.old_header_start(),
        hunk.removed_count,
        hunk.new_header_start(),
        hunk.added_count
    )];
    lines.extend(hunk.removed_lines.iter().map(|line| format!("-{line}")));
    lines.extend(hunk.added_lines.iter().map(|line| format!("+{line}")));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_git::{load, patch_hunks};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn repo(name: &str) -> PathBuf {
        let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kawi_app_git_{}_{}_{}",
            std::process::id(),
            name,
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        command(&dir, &["init", "-q"]);
        command(&dir, &["config", "user.email", "test@example.com"]);
        command(&dir, &["config", "user.name", "Test"]);
        dir
    }

    fn command(dir: &Path, args: &[&str]) {
        Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .unwrap();
    }

    fn write(path: &Path, text: &str) {
        fs::write(path, text).unwrap();
    }

    fn commit(dir: &Path) {
        command(dir, &["add", "."]);
        command(dir, &["commit", "-qm", "base"]);
    }

    fn index_text(dir: &Path, name: &str) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(["show", &format!(":{name}")])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn signs_use_the_gitsigns_changedelete_layout() {
        let file = GitFile {
            root: PathBuf::from("/tmp"),
            path: PathBuf::from("/tmp/a"),
            relpath: "a".to_string(),
            mode: "100644".to_string(),
            untracked: false,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: lines("a\nb\nc\nd\ne"),
            head: lines("a\nb\nc\nd\ne"),
        };
        let session = GitSession::new(file, &lines("a\nX\ne"), 1, false);
        assert_eq!(
            session.visible_signs(0, 5),
            vec![None, Some(GitSign::ChangeDelete), None, None, None]
        );
    }

    #[test]
    fn navigation_moves_to_the_hunk_edges() {
        let file = GitFile {
            root: PathBuf::from("/tmp"),
            path: PathBuf::from("/tmp/a"),
            relpath: "a".to_string(),
            mode: "100644".to_string(),
            untracked: false,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: lines("a\nb\nc\nd\ne\nf"),
            head: lines("a\nb\nc\nd\ne\nf"),
        };
        let session = GitSession::new(file, &lines("a\nB\nc\nd\nE\nf"), 1, false);
        let next = session.next(0).unwrap();
        assert_eq!((next.row, next.index, next.total), (1, 1, 2));
        let next = session.next(next.row).unwrap();
        assert_eq!((next.row, next.index), (4, 2));
        let prev = session.prev(3).unwrap();
        assert_eq!((prev.row, prev.index), (1, 1));
    }

    #[test]
    fn stages_the_hunk_under_the_cursor() {
        let dir = repo("stage");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_hunk(1, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nB\nc\n");
        assert_eq!(
            session.visible_signs(0, 3),
            vec![None, Some(GitSign::StagedChange), None]
        );
    }

    #[test]
    fn staging_a_staged_hunk_unstages_it() {
        let dir = repo("unstage_staged");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc");
        let mut session = GitSession::new(file, &buffer, 1, false);
        assert_eq!(session.stage_hunk(1, &buffer, 1).unwrap(), GitStage::Staged);
        assert_eq!(
            session.visible_signs(0, 3),
            vec![None, Some(GitSign::StagedChange), None]
        );
        assert_eq!(
            session.stage_hunk(1, &buffer, 1).unwrap(),
            GitStage::Unstaged
        );
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\n");
        assert_eq!(
            session.visible_signs(0, 3),
            vec![None, Some(GitSign::Change), None]
        );
    }

    #[test]
    fn undo_stage_restores_the_index() {
        let dir = repo("undo");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_buffer(&buffer, 1).unwrap();
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\n");
    }

    #[test]
    fn staged_signs_remain_after_staging_and_disappear_on_undo() {
        let dir = repo("staged_signs");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_hunk(1, &buffer, 1).unwrap();
        assert_eq!(
            session.visible_signs(0, 3),
            vec![None, Some(GitSign::StagedChange), None]
        );
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(
            session.visible_signs(0, 3),
            vec![None, Some(GitSign::Change), None]
        );
    }

    #[test]
    fn untracked_files_use_the_untracked_sign() {
        let dir = repo("untracked_signs");
        let path = dir.join("new.txt");
        write(&path, "a\nb\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nb");
        let mut session = GitSession::new(file, &buffer, 1, false);
        assert_eq!(
            session.visible_signs(0, 2),
            vec![Some(GitSign::Untracked), Some(GitSign::Untracked)]
        );
        session.stage_buffer(&buffer, 1).unwrap();
        assert_eq!(
            session.visible_signs(0, 2),
            vec![Some(GitSign::StagedAdd), Some(GitSign::StagedAdd)]
        );
    }

    #[test]
    fn undo_stage_removes_an_untracked_index_entry() {
        let dir = repo("untracked_undo");
        let path = dir.join("new.txt");
        write(&path, "a\nb\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nb");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_buffer(&buffer, 1).unwrap();
        session.undo_stage(&buffer, 1).unwrap();
        let output = Command::new("git")
            .current_dir(&dir)
            .args(["diff", "--cached", "--name-only"])
            .output()
            .unwrap();
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn preview_and_diff_include_hunk_content() {
        let file = GitFile {
            root: PathBuf::from("/tmp"),
            path: PathBuf::from("/tmp/a"),
            relpath: "a.txt".to_string(),
            mode: "100644".to_string(),
            untracked: false,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: lines("a\nb\nc"),
            head: lines("a\nb\nc"),
        };
        let session = GitSession::new(file, &lines("a\nB\nc"), 1, false);
        let preview = session.preview_hunk(1).unwrap();
        assert_eq!(preview, vec!["@@ -2,1 +2,1 @@", "-b", "+B"]);
        let diff = session.diff_lines();
        assert!(diff.iter().any(|line| line == "-b"));
        assert!(diff.iter().any(|line| line == "+B"));
    }

    #[test]
    fn stages_a_deletion_hunk() {
        let dir = repo("stage_delete");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\nd\n");
        commit(&dir);
        write(&path, "a\nd\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nd");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_hunk(0, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nd\n");
        assert_eq!(
            session.stage_hunk(0, &buffer, 1).unwrap(),
            GitStage::Unstaged
        );
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\nd\n");
    }

    #[test]
    fn stage_buffer_stages_every_hunk_and_undo_restores_them() {
        let dir = repo("stage_buffer");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\nd\ne\n");
        commit(&dir);
        write(&path, "a\nB\nc\nd\nE\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc\nd\nE");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_buffer(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nB\nc\nd\nE\n");
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\nd\ne\n");
    }

    #[test]
    fn stages_and_undoes_an_unterminated_buffer() {
        let dir = repo("unterminated_buffer");
        let path = dir.join("a.txt");
        write(&path, "a\nb");
        commit(&dir);
        write(&path, "a\nB");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB");
        let mut session = GitSession::new(file, &buffer, 1, true);
        session.stage_hunk(1, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nB");
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb");
    }

    #[test]
    fn stages_an_unterminated_deletion_at_eof() {
        let dir = repo("unterminated_delete");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc");
        commit(&dir);
        write(&path, "a\nb");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nb");
        let mut session = GitSession::new(file, &buffer, 1, true);
        session.stage_hunk(1, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb");
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc");
    }

    #[test]
    fn stages_an_unterminated_insertion_at_eof() {
        let dir = repo("unterminated_add");
        let path = dir.join("a.txt");
        write(&path, "a\nb");
        commit(&dir);
        write(&path, "a\nb\nc");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nb\nc");
        let mut session = GitSession::new(file, &buffer, 1, true);
        session.stage_hunk(2, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc");
        session.undo_stage(&buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb");
    }

    #[test]
    fn staging_a_second_hunk_keeps_the_first_staged() {
        let dir = repo("multi");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\nd\ne\n");
        commit(&dir);
        write(&path, "a\nB\nc\nd\nE\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = lines("a\nB\nc\nd\nE");
        let mut session = GitSession::new(file, &buffer, 1, false);
        session.stage_hunk(1, &buffer, 1).unwrap();
        session.stage_hunk(4, &buffer, 1).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nB\nc\nd\nE\n");
    }

    #[test]
    fn patch_builder_matches_the_session_diff() {
        let file = GitFile {
            root: PathBuf::from("/tmp"),
            path: PathBuf::from("/tmp/a"),
            relpath: "a".to_string(),
            mode: "100644".to_string(),
            untracked: false,
            crlf: false,
            no_final_newline: false,
            head_no_final_newline: false,
            base: lines("a\nb\nc"),
            head: lines("a\nb\nc"),
        };
        let buffer = lines("a\nB\nc");
        let session = GitSession::new(file, &buffer, 1, false);
        let settings = PatchSettings {
            base: &session.base,
            buffer: &buffer,
            crlf: false,
            base_no_final_newline: false,
            buffer_no_final_newline: false,
        };
        let patch = patch_hunks("a", "100644", settings, &session.hunks);
        assert!(patch.contains("@@ -2,1 +2,1 @@"));
    }
}
