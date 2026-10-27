use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct Page {
    slug: String,
    title: String,
    aliases: Vec<String>,
    related: Vec<String>,
    body_start: usize,
    path: PathBuf,
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let help_dir = manifest.join("../../docs/help");
    println!("cargo:rerun-if-changed={}", help_dir.display());
    let mut pages = read_pages(&help_dir);
    pages.sort_by(|left, right| {
        let left_index = left.slug == "index";
        let right_index = right.slug == "index";
        right_index
            .cmp(&left_index)
            .then_with(|| left.slug.cmp(&right.slug))
    });
    let output = render_pages(&pages);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(out_dir.join("pages.rs"), output).expect("write help pages");
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
    Page {
        slug,
        title,
        aliases,
        related,
        body_start,
        path: path.to_path_buf(),
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
