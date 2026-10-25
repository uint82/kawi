use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::command::stderr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFile {
    pub root: PathBuf,
    pub path: PathBuf,
    pub relpath: String,
    pub mode: String,
    pub untracked: bool,
    pub crlf: bool,
    pub no_final_newline: bool,
    pub head_no_final_newline: bool,
    pub base: Vec<String>,
    pub head: Vec<String>,
}

pub fn load(path: &Path) -> Result<Option<GitFile>> {
    let path = absolute_path(path)?;
    let Some(root) = find_root(&path)? else {
        return Ok(None);
    };
    let relpath = relative_path(&root, &path)?;
    let entry = index_entry(&root, &relpath)?;
    let Some(entry) = entry else {
        if is_ignored(&root, &relpath)? {
            return Ok(None);
        }
        let raw = fs::read(&path).unwrap_or_default();
        return Ok(Some(GitFile {
            root,
            mode: file_mode(&path),
            path,
            relpath,
            untracked: true,
            crlf: uses_crlf(&raw),
            no_final_newline: !raw.is_empty() && !raw.ends_with(b"\n"),
            head_no_final_newline: false,
            base: Vec::new(),
            head: Vec::new(),
        }));
    };
    let base = read_blob(&root, &entry.hash)?;
    let Ok(base) = std::str::from_utf8(&base) else {
        return Ok(None);
    };
    let (head, head_no_final_newline) = head_text(&root, &relpath);
    Ok(Some(GitFile {
        root,
        path,
        relpath,
        mode: entry.mode,
        untracked: false,
        crlf: uses_crlf(base.as_bytes()),
        no_final_newline: !base.is_empty() && !base.ends_with('\n'),
        head_no_final_newline,
        base: text_lines(base),
        head,
    }))
}

pub fn ensure_intent_to_add(file: &GitFile) -> Result<()> {
    let output = run_git(
        &file.root,
        &["add", "--intent-to-add", "--", file.relpath.as_str()],
    )?;
    if !output.status.success() {
        bail!("git add --intent-to-add failed: {}", stderr(&output));
    }
    Ok(())
}

pub fn reset_index(root: &Path, relpath: &str) -> Result<()> {
    let output = run_git(root, &["reset", "-q", "--", relpath])?;
    if !output.status.success() {
        bail!("git reset failed: {}", stderr(&output));
    }
    Ok(())
}

pub fn apply_index_patch(root: &Path, patch: &str) -> Result<()> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "apply",
            "--cached",
            "--unidiff-zero",
            "--whitespace=nowarn",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run git apply")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(patch.as_bytes())
            .context("failed to write git patch")?;
    }
    let output = child
        .wait_with_output()
        .context("failed to wait for git apply")?;
    if !output.status.success() {
        bail!("git apply failed: {}", stderr(&output));
    }
    Ok(())
}

fn find_root(path: &Path) -> Result<Option<PathBuf>> {
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("."))
    };
    let output = Command::new("git")
        .arg("-C")
        .arg(start)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;
    if !output.status.success() {
        return Ok(None);
    }
    let root = String::from_utf8(output.stdout).context("git returned a non-UTF-8 root path")?;
    let root = PathBuf::from(root.trim_end_matches(['\r', '\n']));
    let root = fs::canonicalize(&root).unwrap_or(root);
    Ok(Some(root))
}

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

struct IndexEntry {
    mode: String,
    hash: String,
}

fn index_entry(root: &Path, relpath: &str) -> Result<Option<IndexEntry>> {
    let output = run_git(root, &["ls-files", "-s", "-z", "--", relpath])?;
    if !output.status.success() {
        bail!("git ls-files failed: {}", stderr(&output));
    }
    let entries = split_index_entries(&output.stdout);
    if entries.len() > 1 {
        bail!("unmerged path: {relpath}");
    }
    let Some(entry) = entries.first() else {
        return Ok(None);
    };
    let Some((mode, hash, stage)) = parse_index_entry(entry) else {
        bail!("unexpected git index output for {relpath}");
    };
    if stage != b"0" {
        bail!("unmerged path: {relpath}");
    }
    let mode = String::from_utf8(mode.to_vec()).context("git returned a non-UTF-8 mode")?;
    if mode != "100644" && mode != "100755" {
        bail!("unsupported git mode: {mode}");
    }
    let hash = String::from_utf8(hash.to_vec()).context("git returned a non-UTF-8 object id")?;
    Ok(Some(IndexEntry { mode, hash }))
}

fn split_index_entries(stdout: &[u8]) -> Vec<&[u8]> {
    stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn parse_index_entry(entry: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let mut fields = entry.splitn(3, |byte| *byte == b' ');
    let mode = fields.next()?;
    let hash = fields.next()?;
    let rest = fields.next()?;
    let stage = rest.split(|byte| *byte == b'\t').next()?;
    Some((mode, hash, stage))
}

fn head_text(root: &Path, relpath: &str) -> (Vec<String>, bool) {
    let Ok(output) = run_git(root, &["show", &format!("HEAD:{relpath}")]) else {
        return (Vec::new(), false);
    };
    if !output.status.success() {
        return (Vec::new(), false);
    }
    let Ok(text) = std::str::from_utf8(&output.stdout) else {
        return (Vec::new(), false);
    };
    let no_final_newline = !text.is_empty() && !text.ends_with('\n');
    (text_lines(text), no_final_newline)
}

fn read_blob(root: &Path, hash: &str) -> Result<Vec<u8>> {
    let output = run_git(root, &["cat-file", "blob", hash])?;
    if !output.status.success() {
        bail!("git cat-file failed: {}", stderr(&output));
    }
    Ok(output.stdout)
}

fn is_ignored(root: &Path, relpath: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--", relpath])
        .output()
        .context("failed to run git check-ignore")?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => bail!("git check-ignore failed: {}", stderr(&output)),
    }
}

fn run_git(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    crate::command::run(root, args)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(path).context("failed to resolve path")?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    Ok(parent.join(path.file_name().unwrap_or_default()))
}

fn file_mode(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
        {
            return "100755".to_string();
        }
    }
    "100644".to_string()
}

fn uses_crlf(bytes: &[u8]) -> bool {
    bytes.windows(2).any(|window| window == b"\r\n")
        && !bytes
            .windows(2)
            .any(|window| window[1] == b'\n' && window[0] != b'\r')
}

fn text_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_hunks;
    use crate::{patch_hunks, PatchSettings};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn repo(name: &str) -> PathBuf {
        let suffix = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kawi_git_{}_{}_{}",
            std::process::id(),
            name,
            suffix
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["init", "-q"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["config", "user.name", "Test"])
            .status()
            .unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::write(path, text).unwrap();
    }

    fn commit_all(dir: &Path) {
        Command::new("git")
            .current_dir(dir)
            .args(["add", "."])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["commit", "-qm", "base"])
            .status()
            .unwrap();
    }

    fn index_text(dir: &Path, relpath: &str) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(["show", &format!(":{relpath}")])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    fn git_output(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    fn stage(
        file: &mut GitFile,
        base: &[String],
        buffer: &[String],
        buffer_no_final_newline: bool,
    ) -> (Vec<String>, String) {
        let hunks = diff_hunks(base, buffer);
        let settings = PatchSettings {
            base,
            buffer,
            crlf: file.crlf,
            base_no_final_newline: file.no_final_newline,
            buffer_no_final_newline,
        };
        let patch = patch_hunks(&file.relpath, &file.mode, settings, &hunks);
        if file.untracked {
            ensure_intent_to_add(file).unwrap();
            file.untracked = false;
        }
        apply_index_patch(&file.root, &patch).unwrap();
        let next = hunks[0].apply(base, false);
        (next, patch)
    }

    #[test]
    fn missing_repository_returns_none() {
        let dir = std::env::temp_dir().join(format!("kawi_git_plain_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        write(&path, "a\n");
        assert!(load(&path).unwrap().is_none());
    }

    #[test]
    fn tracked_file_uses_index_lines() {
        let dir = repo("tracked");
        let path = dir.join("a.txt");
        write(&path, "a\nb\n");
        commit_all(&dir);
        write(&path, "a\nB\n");
        let file = load(&path).unwrap().unwrap();
        assert_eq!(file.base, vec!["a", "b"]);
        assert!(!file.untracked);
        assert_eq!(file.relpath, "a.txt");
    }

    #[test]
    fn untracked_file_has_an_empty_base() {
        let dir = repo("untracked");
        let path = dir.join("new.txt");
        write(&path, "a\n");
        let file = load(&path).unwrap().unwrap();
        assert!(file.untracked);
        assert!(file.base.is_empty());
    }

    #[test]
    fn ignored_file_has_no_git_file() {
        let dir = repo("ignored");
        write(&dir.join(".gitignore"), "target\n");
        let path = dir.join("target");
        write(&path, "ignored\n");
        assert!(load(&path).unwrap().is_none());
    }

    #[test]
    fn stages_a_change_hunk() {
        let dir = repo("stage_change");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit_all(&dir);
        write(&path, "a\nB\nc\n");
        let mut file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "B".to_string(), "c".to_string()];
        let base = file.base.clone();
        let (next, patch) = stage(&mut file, &base, &buffer, false);
        assert_eq!(next, buffer);
        assert!(patch.contains("@@ -2,1 +2,1 @@\n-b\n+B\n"));
        assert_eq!(index_text(&dir, "a.txt"), "a\nB\nc\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "a\nB\nc\n");
    }

    #[test]
    fn stages_an_insertion_at_eof() {
        let dir = repo("stage_eof");
        let path = dir.join("a.txt");
        write(&path, "a\nb\n");
        commit_all(&dir);
        write(&path, "a\nb\nc\n");
        let mut file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, false);
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\n");
    }

    #[test]
    fn stages_a_path_with_spaces() {
        let dir = repo("space");
        let path = dir.join("foo bar.txt");
        write(&path, "a\nb\n");
        commit_all(&dir);
        write(&path, "a\nB\n");
        let mut file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "B".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, false);
        assert_eq!(index_text(&dir, "foo bar.txt"), "a\nB\n");
    }

    #[test]
    fn stages_an_untracked_file() {
        let dir = repo("stage_untracked");
        let path = dir.join("new.txt");
        write(&path, "a\nb\n");
        let mut file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "b".to_string()];
        let base = file.base.clone();
        let (_, patch) = stage(&mut file, &base, &buffer, false);
        assert!(patch.contains("@@ -0,0 +1,2 @@"));
        assert_eq!(index_text(&dir, "new.txt"), "a\nb\n");
    }

    #[test]
    fn stages_crlf_content_without_changing_line_endings() {
        let dir = repo("crlf");
        let path = dir.join("a.txt");
        fs::write(&path, b"a\r\nb\r\nc\r\n").unwrap();
        commit_all(&dir);
        fs::write(&path, b"a\r\nB\r\nc\r\n").unwrap();
        let mut file = load(&path).unwrap().unwrap();
        assert_eq!(file.base, vec!["a", "b", "c"]);
        let buffer = vec!["a".to_string(), "B".to_string(), "c".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, false);
        assert_eq!(fs::read(&path).unwrap(), b"a\r\nB\r\nc\r\n");
        assert_eq!(index_text(&dir, "a.txt").as_bytes(), b"a\r\nB\r\nc\r\n");
    }

    #[test]
    fn stages_a_change_at_an_unterminated_eof() {
        let dir = repo("no_newline_change");
        let path = dir.join("a.txt");
        fs::write(&path, b"a\nb").unwrap();
        commit_all(&dir);
        fs::write(&path, b"a\nB").unwrap();
        let mut file = load(&path).unwrap().unwrap();
        assert!(file.no_final_newline);
        assert!(file.head_no_final_newline);
        let buffer = vec!["a".to_string(), "B".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, true);
        assert_eq!(index_text(&dir, "a.txt"), "a\nB");
    }

    #[test]
    fn stages_an_insertion_at_an_unterminated_eof() {
        let dir = repo("no_newline_add");
        let path = dir.join("a.txt");
        fs::write(&path, b"a\nb").unwrap();
        commit_all(&dir);
        fs::write(&path, b"a\nb\nc").unwrap();
        let mut file = load(&path).unwrap().unwrap();
        assert!(file.no_final_newline);
        let buffer = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, true);
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc");
    }

    #[test]
    fn stages_a_deletion_at_an_unterminated_eof() {
        let dir = repo("no_newline_delete");
        let path = dir.join("a.txt");
        fs::write(&path, b"a\nb").unwrap();
        commit_all(&dir);
        fs::write(&path, b"a\n").unwrap();
        let mut file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string()];
        let base = file.base.clone();
        stage(&mut file, &base, &buffer, false);
        assert_eq!(index_text(&dir, "a.txt"), "a\n");
    }

    #[test]
    fn reverse_patch_restores_the_previous_index() {
        let dir = repo("reverse");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit_all(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "B".to_string(), "c".to_string()];
        let hunks = diff_hunks(&file.base, &buffer);
        let settings = PatchSettings {
            base: &file.base,
            buffer: &buffer,
            crlf: file.crlf,
            base_no_final_newline: file.no_final_newline,
            buffer_no_final_newline: false,
        };
        let patch = patch_hunks(&file.relpath, &file.mode, settings, &hunks);
        apply_index_patch(&file.root, &patch).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nB\nc\n");
        let reverse: Vec<_> = hunks.iter().map(|hunk| hunk.reversed()).collect();
        let settings = PatchSettings {
            base: &buffer,
            buffer: &file.base,
            crlf: file.crlf,
            base_no_final_newline: false,
            buffer_no_final_newline: file.no_final_newline,
        };
        let patch = patch_hunks(&file.relpath, &file.mode, settings, &reverse);
        apply_index_patch(&file.root, &patch).unwrap();
        assert_eq!(index_text(&dir, "a.txt"), "a\nb\nc\n");
    }

    #[test]
    fn output_matches_git_for_staged_change() {
        let dir = repo("git_diff");
        let path = dir.join("a.txt");
        write(&path, "a\nb\nc\n");
        commit_all(&dir);
        write(&path, "a\nB\nc\n");
        let file = load(&path).unwrap().unwrap();
        let buffer = vec!["a".to_string(), "B".to_string(), "c".to_string()];
        let base = file.base.clone();
        let mut file = file;
        stage(&mut file, &base, &buffer, false);
        let diff = git_output(&dir, &["diff", "--cached", "--unified=0"]);
        assert!(diff.contains("@@ -2 +2 @@"));
        assert!(diff.contains("\n-b\n+B\n"));
    }
}
