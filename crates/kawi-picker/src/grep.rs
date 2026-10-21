use std::io;
use std::path::Path;

use grep_matcher::Matcher;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};

use crate::files::{relative_path, walk_project};

const MAX_LINE_CHARS: usize = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrepHit {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
}

pub fn grep_project(root: &Path, query: &str, limit: usize) -> Result<Vec<GrepHit>, String> {
    grep_project_while(root, query, limit, || true)
}

pub(crate) fn grep_project_while(
    root: &Path,
    query: &str,
    limit: usize,
    keep_going: impl Fn() -> bool,
) -> Result<Vec<GrepHit>, String> {
    let matcher = RegexMatcherBuilder::new()
        .case_smart(true)
        .build(query)
        .map_err(|error| error.to_string())?;
    let mut searcher = SearcherBuilder::new()
        .line_number(true)
        .binary_detection(BinaryDetection::quit(b'\0'))
        .build();
    let mut hits = Vec::new();
    walk_project(root, |path| {
        if hits.len() >= limit || !keep_going() {
            return false;
        }
        let Some(relative) = relative_path(root, path) else {
            return true;
        };
        let mut sink = HitSink {
            path: relative,
            matcher: &matcher,
            hits: &mut hits,
            limit,
            keep_going: &keep_going,
        };
        let _ = searcher.search_path(&matcher, path, &mut sink);
        hits.len() < limit
    });
    hits.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
    });
    Ok(hits)
}

struct HitSink<'a, F> {
    path: String,
    matcher: &'a RegexMatcher,
    hits: &'a mut Vec<GrepHit>,
    limit: usize,
    keep_going: &'a F,
}

impl<F: Fn() -> bool> Sink for HitSink<'_, F> {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        matched: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        if !(self.keep_going)() {
            return Ok(false);
        }
        let line = trim_line(matched.bytes());
        let text = line_text(line);
        let number = matched.line_number().unwrap_or(0) as usize;
        let mut offset = 0;
        while offset <= line.len() {
            let Some(found) = self
                .matcher
                .find_at(line, offset)
                .map_err(io::Error::other)?
            else {
                break;
            };
            if found.start() == found.end() {
                offset = found.end().saturating_add(1);
                continue;
            }
            self.hits.push(GrepHit {
                path: self.path.clone(),
                line: number,
                column: column_at(line, found.start()),
                text: text.clone(),
            });
            if self.hits.len() >= self.limit {
                return Ok(false);
            }
            offset = found.end();
        }
        Ok(true)
    }

    fn binary_data(&mut self, _searcher: &Searcher, _offset: u64) -> Result<bool, io::Error> {
        Ok(false)
    }
}

fn trim_line(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

fn line_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.chars().take(MAX_LINE_CHARS).collect()
}

fn column_at(line: &[u8], byte: usize) -> usize {
    String::from_utf8_lossy(&line[..byte])
        .chars()
        .count()
        .saturating_add(1)
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
        let root = std::env::temp_dir().join(format!("kawi_grep_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).expect("project");
        fs::create_dir_all(root.join(".git")).expect("git");
        fs::write(root.join(".gitignore"), "ignored.txt\n").expect("ignore");
        fs::write(root.join("alpha.txt"), "Needle one\nneedle two\n").expect("alpha");
        fs::write(root.join("ignored.txt"), "needle ignored\n").expect("ignored");
        fs::write(root.join("sub/beta.txt"), "é needle\n").expect("beta");
        root
    }

    #[test]
    fn finds_matches_with_line_and_character_columns() {
        let root = project("columns");
        let hits = grep_project(&root, "needle", 20).expect("grep");
        assert_eq!(
            hits,
            vec![
                GrepHit {
                    path: "alpha.txt".to_string(),
                    line: 1,
                    column: 1,
                    text: "Needle one".to_string(),
                },
                GrepHit {
                    path: "alpha.txt".to_string(),
                    line: 2,
                    column: 1,
                    text: "needle two".to_string(),
                },
                GrepHit {
                    path: "sub/beta.txt".to_string(),
                    line: 1,
                    column: 3,
                    text: "é needle".to_string(),
                },
            ]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn respects_gitignore_and_skips_binary_files() {
        let root = project("ignore");
        fs::write(root.join("binary.bin"), b"needle\0needle").expect("binary");
        let hits = grep_project(&root, "needle", 20).expect("grep");
        assert!(hits.iter().all(|hit| hit.path != "ignored.txt"));
        assert!(hits.iter().all(|hit| hit.path != "binary.bin"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn smart_case_uses_uppercase_queries_sensitively() {
        let root = project("case");
        let lower = grep_project(&root, "needle", 20).expect("lower");
        let upper = grep_project(&root, "Needle", 20).expect("upper");
        assert_eq!(lower.len(), 3);
        assert_eq!(upper.len(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn invalid_regex_reports_an_error() {
        let root = project("invalid");
        assert!(grep_project(&root, "[", 20).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn limit_stops_collecting_matches() {
        let root = project("limit");
        let hits = grep_project(&root, "needle", 1).expect("grep");
        assert_eq!(hits.len(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
