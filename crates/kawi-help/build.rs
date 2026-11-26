use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CODE_PAGE: &str = "errors";
const FENCE: &str = "```";

struct Page {
    slug: String,
    title: String,
    aliases: Vec<String>,
    related: Vec<String>,
    codes: Vec<String>,
    body_start: usize,
    path: PathBuf,
}

struct CodeFile {
    code: String,
    page: String,
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let help_dir = manifest.join("../../docs/help");
    println!("cargo:rerun-if-changed={}", help_dir.display());
    let mut pages = read_pages(&help_dir);
    let codes = collect_codes(&pages);
    assert_no_code_topic_collisions(&pages, &codes);
    sort_pages(&mut pages);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(out_dir.join("pages.rs"), render_pages(&pages)).expect("write help pages");
    fs::write(out_dir.join("codes.rs"), render_codes(&codes)).expect("write help codes");
}

fn read_pages(dir: &Path) -> Vec<Page> {
    let mut pages = Vec::new();
    let entries = fs::read_dir(dir).expect("read docs/help");
    for entry in entries {
        let path = entry.expect("read help entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        pages.push(read_page(&path));
    }
    assert!(!pages.is_empty(), "docs/help has no pages");
    pages
}

fn read_page(path: &Path) -> Page {
    let text = fs::read_to_string(path).expect("read help page");
    let slug = path
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("help page name")
        .to_string();
    let (meta, body_start) = frontmatter(&text);
    let title = meta
        .iter()
        .find(|(key, _)| key == "title")
        .map(|(_, value)| scalar(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| title_from_slug(&slug));
    let aliases = meta
        .iter()
        .find(|(key, _)| key == "aliases")
        .map(|(_, value)| array(value))
        .unwrap_or_default();
    let related = meta
        .iter()
        .find(|(key, _)| key == "related")
        .map(|(_, value)| array(value))
        .unwrap_or_default();
    let codes = code_headings(&text[body_start..]);
    Page {
        slug,
        title,
        aliases,
        related,
        codes,
        body_start,
        path: path.to_path_buf(),
    }
}

fn sort_pages(pages: &mut [Page]) {
    pages.sort_by(|left, right| {
        let left_index = left.slug == "index";
        let right_index = right.slug == "index";
        right_index
            .cmp(&left_index)
            .then_with(|| left.slug.cmp(&right.slug))
    });
}

fn code_headings(body: &str) -> Vec<String> {
    let mut codes = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with(FENCE) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((code, _)) = code_heading(line) {
            codes.push(code.to_string());
        }
    }
    codes
}

fn code_heading(line: &str) -> Option<(&str, &str)> {
    let (level, heading) = heading(line)?;
    let code = heading.split_once(':').map_or_else(
        || heading.split_whitespace().next().unwrap_or_default(),
        |(code, _)| code.trim(),
    );
    if !is_diagnostic_code(code) {
        return None;
    }
    assert_eq!(level, 3, "help code {code} must use '### CODE: Title'");
    let (_, title) = heading
        .split_once(':')
        .unwrap_or_else(|| panic!("help code {code} needs 'CODE: Title'"));
    let title = title.trim();
    assert!(!title.is_empty(), "help code {code} needs a title");
    Some((code, title))
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if level == 0 {
        return None;
    }
    Some((level, trimmed[level..].trim()))
}

fn is_diagnostic_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    matches!(bytes.first(), Some(b'E' | b'K' | b'W'))
        && (2..=5).contains(&bytes.len())
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

fn collect_codes(pages: &[Page]) -> Vec<CodeFile> {
    let mut codes = BTreeMap::new();
    for page in pages {
        for code in &page.codes {
            assert_eq!(
                page.slug, CODE_PAGE,
                "help code {code} must live in {CODE_PAGE}.md"
            );
            assert!(
                codes.insert(code.clone(), page.slug.clone()).is_none(),
                "duplicate help code {code}"
            );
        }
    }
    assert!(
        !codes.is_empty(),
        "docs/help/{CODE_PAGE}.md defines no codes"
    );
    codes
        .into_iter()
        .map(|(code, page)| CodeFile { code, page })
        .collect()
}

fn assert_no_code_topic_collisions(pages: &[Page], codes: &[CodeFile]) {
    for page in pages {
        for topic in std::iter::once(&page.slug)
            .chain(page.aliases.iter())
            .chain(std::iter::once(&page.title))
        {
            if let Some(entry) = codes
                .iter()
                .find(|entry| entry.code.eq_ignore_ascii_case(topic))
            {
                panic!("help topic {topic} collides with code {}", entry.code);
            }
        }
    }
}

fn frontmatter(text: &str) -> (Vec<(String, String)>, usize) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (Vec::new(), 0);
    };
    let Some(end) = rest.find("\n---\n") else {
        return (Vec::new(), 0);
    };
    let header = &rest[..end];
    let body_start = 4 + end + 5;
    let meta = header
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();
    (meta, body_start)
}

fn scalar(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn array(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(scalar)
        .filter(|value| !value.is_empty())
        .collect()
}

fn title_from_slug(slug: &str) -> String {
    let mut title = slug.replace('-', " ");
    if let Some(first) = title.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    title
}

fn render_pages(pages: &[Page]) -> String {
    let mut output = String::from(
        "pub(crate) struct PageFile {\n    pub slug: &'static str,\n    pub title: &'static str,\n    pub aliases: &'static [&'static str],\n    pub related: &'static [&'static str],\n    pub body: &'static str,\n    pub body_start: usize,\n}\n\npub(crate) const PAGE_FILES: &[PageFile] = &[\n",
    );
    for page in pages {
        let name = page
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("help page file name");
        output.push_str("    PageFile {\n");
        output.push_str(&format!("        slug: {:?},\n", page.slug));
        output.push_str(&format!("        title: {:?},\n", page.title));
        output.push_str(&format!("        aliases: &{:?},\n", page.aliases));
        output.push_str(&format!("        related: &{:?},\n", page.related));
        output.push_str(&format!(
            "        body: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../docs/help/{name}\")),\n"
        ));
        output.push_str(&format!("        body_start: {}usize,\n", page.body_start));
        output.push_str("    },\n");
    }
    output.push_str("];\n");
    output
}

fn render_codes(codes: &[CodeFile]) -> String {
    let mut output = String::from(
        "pub(crate) struct CodeFile {\n    pub code: &'static str,\n    pub page: &'static str,\n}\n\npub(crate) const CODE_FILES: &[CodeFile] = &[\n",
    );
    for entry in codes {
        output.push_str("    CodeFile {\n");
        output.push_str(&format!("        code: {:?},\n", entry.code));
        output.push_str(&format!("        page: {:?},\n", entry.page));
        output.push_str("    },\n");
    }
    output.push_str("];\n");
    output
}
