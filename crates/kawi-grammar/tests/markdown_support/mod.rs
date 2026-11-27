#![allow(dead_code)]

use std::sync::OnceLock;
use std::time::Duration;

use kawi_grammar::{GrammarLoader, LoadedGrammar, Pack};
use tree_sitter::Language;

use super::support::{fixture_markdown_inline_pack, fixture_markdown_pack};

pub const PARSE_BUDGET: Duration = Duration::from_secs(10);

pub const MARKDOWN_SOURCE: &str = r#"---
title: Kawi
tags:
  - editor
---

# Title

A paragraph with *italic*, **bold**, `code`, and a [link](https://example.com).

> A quote
> - nested item
> - [x] done

1. first
2. second

| Name | Value |
| ---- | ----- |
| one  | two   |

```rust
fn main() {
    println!("hello");
}
```

```javascript
const value = 1;
```

```html
<script>const value = 1;</script>
```

<div>html block</div>

Inline <span>inside</span> tail.

Setext
======

Unicode café.
"#;

pub const TOML_METADATA_SOURCE: &str = "+++\ntitle = \"Kawi\"\n+++\n\n# Head\n";

pub const ERROR_SOURCE: &str = "```";

pub const INLINE_SOURCE: &str = "Some *italic* and **bold** and `code` with [label](https://example.com) and ![alt](image.png). <span>inside</span> tail.\n";

pub const INLINE_ERROR_SOURCE: &str = "__]~~[-";

pub const INLINE_LATEX_SOURCE: &str = "Some $x + y$ and $$z$$ text.\n";

pub const CORPUS: &[(&str, &str)] = &[
    ("features", MARKDOWN_SOURCE),
    ("toml_metadata", TOML_METADATA_SOURCE),
    ("errors", ERROR_SOURCE),
    ("empty", ""),
    ("crlf", "# Title\r\n\r\n- one\r\n- two\r\n"),
];

pub const INLINE_CORPUS: &[(&str, &str)] = &[
    ("features", INLINE_SOURCE),
    ("latex", INLINE_LATEX_SOURCE),
    ("errors", INLINE_ERROR_SOURCE),
    ("empty", ""),
    ("crlf", "Some *italic* text.\r\nNext line.\r\n"),
];

pub const HIGHLIGHTS: &str = include_str!("../../packs/markdown/0.5.4/queries/highlights.scm");
pub const INJECTIONS: &str = include_str!("../../packs/markdown/0.5.4/queries/injections.scm");
pub const INLINE_HIGHLIGHTS: &str =
    include_str!("../../packs/markdown_inline/0.5.4/queries/highlights.scm");
pub const INLINE_INJECTIONS: &str =
    include_str!("../../packs/markdown_inline/0.5.4/queries/injections.scm");

pub const NATIVE_HIGHLIGHTS: &str = include_str!("../../../kawi-syntax/queries/markdown.scm");
pub const NATIVE_INJECTIONS: &str =
    include_str!("../../../kawi-syntax/queries/markdown-injections.scm");
pub const NATIVE_INLINE_HIGHLIGHTS: &str =
    include_str!("../../../kawi-syntax/queries/markdown-inline.scm");
pub const NATIVE_INLINE_INJECTIONS: &str =
    include_str!("../../../kawi-syntax/queries/markdown-inline-injections.scm");

pub fn grammar() -> &'static LoadedGrammar {
    static GRAMMAR: OnceLock<LoadedGrammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| load(fixture_markdown_pack(), "Markdown"))
}

pub fn inline_grammar() -> &'static LoadedGrammar {
    static GRAMMAR: OnceLock<LoadedGrammar> = OnceLock::new();
    GRAMMAR.get_or_init(|| load(fixture_markdown_inline_pack(), "Markdown inline"))
}

pub fn native_language() -> Language {
    tree_sitter_md::LANGUAGE.into()
}

pub fn native_inline_language() -> Language {
    tree_sitter_md::INLINE_LANGUAGE.into()
}

pub fn large_markdown_source(blocks: usize) -> String {
    use std::fmt::Write;

    let mut source = String::with_capacity(blocks * 96);
    for index in 0..blocks {
        writeln!(source, "## Section {index}").expect("write source");
        writeln!(source, "Paragraph {index} with *italic* and `code`.").expect("write source");
        writeln!(source, "- item {index}").expect("write source");
        writeln!(source).expect("write source");
    }
    source
}

pub fn large_inline_source(segments: usize) -> String {
    use std::fmt::Write;

    let mut source = String::with_capacity(segments * 48);
    for index in 0..segments {
        write!(source, "Text *{index}* with **bold** and `code`. ").expect("write source");
    }
    source.push('\n');
    source
}

fn load(path: std::path::PathBuf, name: &str) -> LoadedGrammar {
    let pack = Pack::open(path).unwrap_or_else(|error| panic!("open {name} pack: {error}"));
    GrammarLoader::new()
        .unwrap_or_else(|error| panic!("create loader: {error}"))
        .load(&pack)
        .unwrap_or_else(|error| panic!("load {name} pack: {error}"))
}
