use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use regex::{Captures, Regex, RegexBuilder};

use crate::grep::grep_project_while;
use crate::smartcase::case_insensitive;

pub const MAX_REPLACE_MATCHES: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacePreview {
    pub files: Vec<ReplaceFile>,
    pub entries: Vec<ReplaceEntry>,
    pub matches: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceFile {
    pub path: String,
    pub content: String,
    pub original_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceEntry {
    pub path: String,
    pub line: usize,
    pub before: String,
    pub after: String,
}

pub fn replace_preview(
    root: &Path,
    query: &str,
    replacement: &str,
    limit: usize,
) -> Result<ReplacePreview, String> {
    replace_preview_while(root, query, replacement, limit, || true)
}

pub(crate) fn replace_preview_while(
    root: &Path,
    query: &str,
    replacement: &str,
    limit: usize,
    keep_going: impl Fn() -> bool,
) -> Result<ReplacePreview, String> {
    if query.is_empty() {
        return Err("Type a search pattern first".to_string());
    }
    let hits = grep_project_while(root, query, limit.saturating_add(1), &keep_going)?;
    if hits.is_empty() {
        return Err("No matches".to_string());
    }
    if hits.len() > limit {
        return Err(over_limit_error(limit));
    }
    let regex = build_regex(query)?;
    let lines = hit_lines(&hits);
    let mut preview = ReplacePreview {
        files: Vec::new(),
        entries: Vec::new(),
        matches: 0,
    };
    for (path, line_numbers) in lines {
        if preview.matches > limit || !keep_going() {
            break;
        }
        let Some((file, entries, matches)) =
            replace_file(root, &path, &line_numbers, &regex, replacement)
        else {
            continue;
        };
        preview.matches = preview.matches.saturating_add(matches);
        if let Some(file) = file {
            preview.files.push(file);
            preview.entries.extend(entries);
        }
    }
    if preview.matches == 0 {
        return Err("No matches".to_string());
    }
    if preview.matches > limit {
        return Err(over_limit_error(limit));
    }
    if preview.files.is_empty() {
        return Err("No changes".to_string());
    }
    preview
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    preview
        .entries
        .sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));
    Ok(preview)
}

fn hit_lines(hits: &[crate::grep::GrepHit]) -> BTreeMap<String, BTreeSet<usize>> {
    let mut lines: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for hit in hits {
        lines.entry(hit.path.clone()).or_default().insert(hit.line);
    }
    lines
}

fn over_limit_error(limit: usize) -> String {
    format!("More than {limit} matches; narrow the search before replacing")
}

fn build_regex(query: &str) -> Result<Regex, String> {
    RegexBuilder::new(query)
        .case_insensitive(case_insensitive(query))
        .build()
        .map_err(|error| error.to_string())
}

fn replace_file(
    root: &Path,
    relative: &str,
    line_numbers: &BTreeSet<usize>,
    regex: &Regex,
    replacement: &str,
) -> Option<(Option<ReplaceFile>, Vec<ReplaceEntry>, usize)> {
    let bytes = fs::read(root.join(relative)).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let content = String::from_utf8(bytes).ok()?;
    let original_hash = content_hash(content.as_bytes());
    let mut changed = false;
    let mut matches = 0usize;
    let mut entries = Vec::new();
    let mut replaced = String::with_capacity(content.len());
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let number = index.saturating_add(1);
        if !line_numbers.contains(&number) {
            replaced.push_str(line);
            continue;
        }
        let (body, ending) = split_line_ending(line);
        let (after, count) = replace_line(body, regex, replacement);
        matches = matches.saturating_add(count);
        if after == body {
            replaced.push_str(line);
            continue;
        }
        changed = true;
        replaced.push_str(&after);
        replaced.push_str(ending);
        entries.push(ReplaceEntry {
            path: relative.to_string(),
            line: number,
            before: body.to_string(),
            after,
        });
    }
    if !changed {
        return Some((None, entries, matches));
    }
    Some((
        Some(ReplaceFile {
            path: relative.to_string(),
            content: replaced,
            original_hash,
        }),
        entries,
        matches,
    ))
}

fn replace_line(body: &str, regex: &Regex, replacement: &str) -> (String, usize) {
    let mut result = String::new();
    let mut position = 0usize;
    let mut count = 0usize;
    for captures in regex.captures_iter(body) {
        let Some(found) = captures.get(0) else {
            continue;
        };
        if found.start() == found.end() {
            continue;
        }
        result.push_str(&body[position..found.start()]);
        result.push_str(&expand_replacement(replacement, &captures));
        position = found.end();
        count = count.saturating_add(1);
    }
    result.push_str(&body[position..]);
    (result, count)
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        return (body, "\r\n");
    }
    if let Some(body) = line.strip_suffix('\n') {
        return (body, "\n");
    }
    (line, "")
}

pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn expand_replacement(template: &str, captures: &Captures<'_>) -> String {
    let mut result = String::new();
    let mut characters = template.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '&' => push_capture(&mut result, captures, 0),
            '\\' => match characters.next() {
                Some(digit @ '0'..='9') => {
                    let group = digit.to_digit(10).unwrap_or(0) as usize;
                    push_capture(&mut result, captures, group);
                }
                Some('&') => result.push('&'),
                Some('$') => result.push('$'),
                Some('\\') => result.push('\\'),
                Some(other) => result.push(other),
                None => result.push('\\'),
            },
            '$' => match characters.peek().copied() {
                Some('$') => {
                    characters.next();
                    result.push('$');
                }
                Some('{') => {
                    characters.next();
                    let mut name = String::new();
                    for next in characters.by_ref() {
                        if next == '}' {
                            break;
                        }
                        name.push(next);
                    }
                    push_named_capture(&mut result, captures, &name);
                }
                Some(next) if next.is_ascii_digit() => {
                    let mut digits = String::new();
                    while let Some(digit) = characters.peek().copied() {
                        if !digit.is_ascii_digit() {
                            break;
                        }
                        digits.push(digit);
                        characters.next();
                    }
                    push_capture(&mut result, captures, digits.parse().unwrap_or(0));
                }
                Some(next) if is_capture_name_start(next) => {
                    let mut name = String::new();
                    while let Some(next) = characters.peek().copied() {
                        if !is_capture_name(next) {
                            break;
                        }
                        name.push(next);
                        characters.next();
                    }
                    push_named_capture(&mut result, captures, &name);
                }
                _ => result.push('$'),
            },
            _ => result.push(character),
        }
    }
    result
}

fn push_capture(result: &mut String, captures: &Captures<'_>, index: usize) {
    if let Some(capture) = captures.get(index) {
        result.push_str(capture.as_str());
    }
}

fn push_named_capture(result: &mut String, captures: &Captures<'_>, name: &str) {
    if let Some(capture) = captures.name(name) {
        result.push_str(capture.as_str());
    }
}

fn is_capture_name_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_capture_name(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn project(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_replace_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("project");
        root
    }

    #[test]
    fn builds_a_preview_with_groups_and_counts() {
        let root = project("groups");
        fs::write(root.join("a.txt"), "foo foo\nbar\nFOO\n").expect("file");
        fs::write(root.join("b.txt"), "foo\n").expect("file");
        let preview = replace_preview(&root, "(foo) (foo)", "$2-$1", 10).expect("preview");
        assert_eq!(preview.matches, 1);
        assert_eq!(preview.files.len(), 1);
        assert_eq!(
            preview.entries,
            vec![ReplaceEntry {
                path: "a.txt".to_string(),
                line: 1,
                before: "foo foo".to_string(),
                after: "foo-foo".to_string(),
            }]
        );
        assert_eq!(preview.files[0].content, "foo-foo\nbar\nFOO\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn supports_vim_and_ripgrep_group_syntax() {
        let root = project("groups_syntax");
        fs::write(root.join("a.txt"), "alpha beta\n").expect("file");
        let vim = replace_preview(&root, "(alpha) (beta)", "\\2-\\1", 10).expect("vim");
        assert_eq!(vim.files[0].content, "beta-alpha\n");
        let ripgrep = replace_preview(
            &root,
            "(?P<first>alpha) (?P<second>beta)",
            "$second-$first",
            10,
        )
        .expect("ripgrep");
        assert_eq!(ripgrep.files[0].content, "beta-alpha\n");
        let whole = replace_preview(&root, "alpha", "[&]", 10).expect("whole");
        assert_eq!(whole.files[0].content, "[alpha] beta\n");
        let literal = replace_preview(&root, "alpha", "\\&", 10).expect("literal");
        assert_eq!(literal.files[0].content, "& beta\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn lower_case_queries_are_case_insensitive() {
        let root = project("smartcase");
        fs::write(root.join("a.txt"), "foo foo\nbar\nFOO\n").expect("file");
        let preview = replace_preview(&root, "foo", "x", 10).expect("preview");
        assert_eq!(preview.matches, 3);
        assert_eq!(preview.files[0].content, "x x\nbar\nx\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn upper_case_queries_are_case_sensitive() {
        let root = project("smartcase_upper");
        fs::write(root.join("a.txt"), "foo FOO\n").expect("file");
        let preview = replace_preview(&root, "FOO", "x", 10).expect("preview");
        assert_eq!(preview.matches, 1);
        assert_eq!(preview.files[0].content, "foo x\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn smartcase_without_a_literal_stays_case_sensitive() {
        let root = project("smartcase_property");
        fs::write(root.join("a.txt"), "a\nA\n").expect("file");
        let preview = replace_preview(&root, r"\p{Lu}", "X", 10).expect("preview");
        assert_eq!(preview.matches, 1);
        assert_eq!(preview.files[0].content, "a\nX\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_results_over_the_limit() {
        let root = project("limit");
        fs::write(root.join("a.txt"), "foo foo foo\n").expect("file");
        assert!(replace_preview(&root, "foo", "x", 2).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preserves_crlf_and_missing_final_newline() {
        let root = project("newlines");
        fs::write(root.join("a.txt"), "foo\r\nbar").expect("file");
        let preview = replace_preview(&root, "foo", "x", 10).expect("preview");
        assert_eq!(preview.files[0].content, "x\r\nbar");
        assert_eq!(preview.entries[0].before, "foo");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_invalid_and_empty_patterns() {
        let root = project("invalid");
        assert_eq!(
            replace_preview(&root, "", "x", 10),
            Err("Type a search pattern first".to_string())
        );
        assert!(replace_preview(&root, "(", "x", 10).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn hash_changes_with_content() {
        assert_ne!(content_hash(b"a"), content_hash(b"b"));
    }
}
