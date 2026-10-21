use crate::Hunk;

#[derive(Clone, Copy)]
pub struct PatchSettings<'a> {
    pub base: &'a [String],
    pub buffer: &'a [String],
    pub crlf: bool,
    pub base_no_final_newline: bool,
    pub buffer_no_final_newline: bool,
}

pub fn patch_hunks(
    relpath: &str,
    mode: &str,
    settings: PatchSettings<'_>,
    hunks: &[Hunk],
) -> String {
    let old_path = format!("a/{relpath}");
    let new_path = format!("b/{relpath}");
    let mut patch = String::new();
    patch.push_str(&format!(
        "diff --git {} {}\n",
        quote_patch_path(&old_path),
        quote_patch_path(&new_path)
    ));
    patch.push_str(&format!("index 000000..000000 {mode}\n"));
    patch.push_str(&format!("--- {}\n", quote_patch_path(&old_path)));
    patch.push_str(&format!("+++ {}\n", quote_patch_path(&new_path)));
    for hunk in hunks {
        append_hunk(&mut patch, hunk, settings);
    }
    patch
}

fn append_hunk(patch: &mut String, hunk: &Hunk, settings: PatchSettings<'_>) {
    if is_insertion_at_unterminated_eof(hunk, settings) {
        append_eof_insertion(patch, hunk, settings);
        return;
    }
    if is_deletion_at_unterminated_eof(hunk, settings) {
        append_eof_deletion(patch, hunk, settings);
        return;
    }
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_header_start(),
        hunk.removed_count,
        hunk.new_header_start(),
        hunk.added_count
    ));
    append_lines(patch, '-', &hunk.removed_lines, settings.crlf);
    append_eof_marker(
        patch,
        settings.base_no_final_newline && removed_reaches_eof(hunk, settings.base.len()),
    );
    append_lines(patch, '+', &hunk.added_lines, settings.crlf);
    append_eof_marker(
        patch,
        settings.buffer_no_final_newline && added_reaches_eof(hunk, settings.buffer.len()),
    );
}

fn is_insertion_at_unterminated_eof(hunk: &Hunk, settings: PatchSettings<'_>) -> bool {
    settings.base_no_final_newline
        && !settings.base.is_empty()
        && hunk.removed_count == 0
        && hunk.added_count > 0
        && hunk.removed_start == settings.base.len()
}

fn append_eof_insertion(patch: &mut String, hunk: &Hunk, settings: PatchSettings<'_>) {
    let line = settings.base.len();
    patch.push_str(&format!(
        "@@ -{},1 +{},{} @@\n",
        line,
        line,
        hunk.added_count.saturating_add(1)
    ));
    let last = settings.base.last().map_or("", String::as_str);
    append_lines(patch, '-', &[last.to_string()], settings.crlf);
    append_eof_marker(patch, true);
    append_lines(patch, '+', &[last.to_string()], settings.crlf);
    append_lines(patch, '+', &hunk.added_lines, settings.crlf);
    append_eof_marker(
        patch,
        settings.buffer_no_final_newline && added_reaches_eof(hunk, settings.buffer.len()),
    );
}

fn is_deletion_at_unterminated_eof(hunk: &Hunk, settings: PatchSettings<'_>) -> bool {
    settings.buffer_no_final_newline
        && !settings.buffer.is_empty()
        && hunk.added_count == 0
        && removed_reaches_eof(hunk, settings.base.len())
}

fn append_eof_deletion(patch: &mut String, hunk: &Hunk, settings: PatchSettings<'_>) {
    let line = hunk.removed_start.saturating_sub(1);
    patch.push_str(&format!(
        "@@ -{},{} +{},1 @@\n",
        line,
        hunk.removed_count.saturating_add(1),
        settings.buffer.len()
    ));
    if let Some(previous) = settings.base.get(line.saturating_sub(1)) {
        append_lines(patch, '-', std::slice::from_ref(previous), settings.crlf);
    }
    append_lines(patch, '-', &hunk.removed_lines, settings.crlf);
    append_eof_marker(patch, settings.base_no_final_newline);
    if let Some(last) = settings.buffer.last() {
        append_lines(patch, '+', std::slice::from_ref(last), settings.crlf);
    }
    append_eof_marker(patch, true);
}

fn append_eof_marker(patch: &mut String, marked: bool) {
    if marked {
        patch.push_str("\\ No newline at end of file\n");
    }
}

fn removed_reaches_eof(hunk: &Hunk, base_len: usize) -> bool {
    hunk.removed_count > 0
        && hunk.removed_start.saturating_add(hunk.removed_count) == base_len.saturating_add(1)
}

fn added_reaches_eof(hunk: &Hunk, buffer_len: usize) -> bool {
    hunk.added_count > 0
        && hunk.added_start.saturating_add(hunk.added_count) == buffer_len.saturating_add(1)
}

fn append_lines(patch: &mut String, prefix: char, lines: &[String], crlf: bool) {
    for line in lines {
        patch.push(prefix);
        patch.push_str(line);
        if crlf {
            patch.push('\r');
        }
        patch.push('\n');
    }
}

fn quote_patch_path(path: &str) -> String {
    if path
        .bytes()
        .all(|byte| byte == b' ' || (0x21..=0x7e).contains(&byte) && byte != b'"' && byte != b'\\')
    {
        return path.to_string();
    }
    let mut quoted = String::from("\"");
    for byte in path.bytes() {
        match byte {
            b'\\' => quoted.push_str("\\\\"),
            b'"' => quoted.push_str("\\\""),
            b'\t' => quoted.push_str("\\t"),
            b'\n' => quoted.push_str("\\n"),
            b'\r' => quoted.push_str("\\r"),
            0x20..=0x7e => quoted.push(byte as char),
            _ => quoted.push_str(&format!("\\{byte:03o}")),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_hunks;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    fn settings<'a>(base: &'a [String], buffer: &'a [String]) -> PatchSettings<'a> {
        PatchSettings {
            base,
            buffer,
            crlf: false,
            base_no_final_newline: false,
            buffer_no_final_newline: false,
        }
    }

    #[test]
    fn change_patch_has_git_headers_and_zero_context() {
        let base = lines("one\ntwo\nthree");
        let buffer = lines("one\nTWO\nthree");
        let hunks = diff_hunks(&base, &buffer);
        let patch = patch_hunks("src/a.txt", "100644", settings(&base, &buffer), &hunks);
        assert_eq!(
            patch,
            "diff --git a/src/a.txt b/src/a.txt\n\
             index 000000..000000 100644\n\
             --- a/src/a.txt\n\
             +++ b/src/a.txt\n\
             @@ -2,1 +2,1 @@\n\
             -two\n\
             +TWO\n"
        );
    }

    #[test]
    fn insertion_patch_uses_the_line_before_the_insertion() {
        let base = lines("a\nb");
        let buffer = lines("a\nb\nc");
        let hunks = diff_hunks(&base, &buffer);
        let patch = patch_hunks("a.txt", "100644", settings(&base, &buffer), &hunks);
        assert!(patch.contains("@@ -2,0 +3,1 @@\n+c\n"));
    }

    #[test]
    fn deletion_patch_uses_the_existing_line() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nc");
        let hunks = diff_hunks(&base, &buffer);
        let patch = patch_hunks("a.txt", "100644", settings(&base, &buffer), &hunks);
        assert!(patch.contains("@@ -2,1 +1,0 @@\n-b\n"));
    }

    #[test]
    fn paths_are_quoted_for_git_apply() {
        assert_eq!(quote_patch_path("a/foo bar.txt"), "a/foo bar.txt");
        assert_eq!(quote_patch_path("a/quote\"x.txt"), "\"a/quote\\\"x.txt\"");
        assert_eq!(quote_patch_path("a/tab\tx.txt"), "\"a/tab\\tx.txt\"");
    }

    #[test]
    fn reversed_patch_swaps_the_sides() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nB\nc");
        let hunks = diff_hunks(&base, &buffer);
        let reversed: Vec<Hunk> = hunks.iter().map(Hunk::reversed).collect();
        let patch = patch_hunks("a.txt", "100644", settings(&base, &buffer), &reversed);
        assert!(patch.contains("--- a/a.txt\n+++ b/a.txt\n"));
        assert!(patch.contains("@@ -2,1 +2,1 @@\n-B\n+b\n"));
    }

    #[test]
    fn unterminated_change_marks_the_old_side() {
        let base = lines("a\nb");
        let buffer = lines("a\nB");
        let hunks = diff_hunks(&base, &buffer);
        let settings = PatchSettings {
            base: &base,
            buffer: &buffer,
            crlf: false,
            base_no_final_newline: true,
            buffer_no_final_newline: false,
        };
        let patch = patch_hunks("a.txt", "100644", settings, &hunks);
        assert!(patch.contains("-b\n\\ No newline at end of file\n+B\n"));
    }

    #[test]
    fn unterminated_change_marks_both_sides() {
        let base = lines("a\nb");
        let buffer = lines("a\nB");
        let hunks = diff_hunks(&base, &buffer);
        let settings = PatchSettings {
            base: &base,
            buffer: &buffer,
            crlf: false,
            base_no_final_newline: true,
            buffer_no_final_newline: true,
        };
        let patch = patch_hunks("a.txt", "100644", settings, &hunks);
        assert!(
            patch.contains("-b\n\\ No newline at end of file\n+B\n\\ No newline at end of file\n")
        );
    }

    #[test]
    fn unterminated_deletion_rewrites_the_new_last_line() {
        let base = lines("a\nb\nc");
        let buffer = lines("a\nb");
        let hunks = diff_hunks(&base, &buffer);
        let settings = PatchSettings {
            base: &base,
            buffer: &buffer,
            crlf: false,
            base_no_final_newline: false,
            buffer_no_final_newline: true,
        };
        let patch = patch_hunks("a.txt", "100644", settings, &hunks);
        assert!(patch.contains("@@ -2,2 +2,1 @@\n-b\n-c\n+b\n\\ No newline at end of file\n"));
    }

    #[test]
    fn unterminated_insertion_rewrites_the_last_line() {
        let base = lines("a\nb");
        let buffer = lines("a\nb\nc");
        let hunks = diff_hunks(&base, &buffer);
        let settings = PatchSettings {
            base: &base,
            buffer: &buffer,
            crlf: false,
            base_no_final_newline: true,
            buffer_no_final_newline: false,
        };
        let patch = patch_hunks("a.txt", "100644", settings, &hunks);
        assert!(patch.contains("@@ -2,1 +2,2 @@\n-b\n\\ No newline at end of file\n+b\n+c\n"));
    }
}
