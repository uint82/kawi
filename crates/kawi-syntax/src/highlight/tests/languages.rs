use super::support::{highlight, name_at, name_of};
use super::*;

fn name_of_last(
    spans: &[SyntaxSpan],
    text: &str,
    row: usize,
    needle: &str,
) -> Option<&'static str> {
    let column = text.lines().nth(row)?.rfind(needle)?;
    name_at(spans, row, column)
}

#[test]
fn highlights_nested_python_types() {
    let text = "class Item:\n    def load(self, values: list[int], mapping: dict[str, int]) -> typing.List[str]:\n        return values\n";
    let spans = highlight(LanguageId::Python, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "Item"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 1, "values"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "list"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "dict"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "typing"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "return"), Some("keyword.return"));
}

#[test]
fn highlights_type_parameters() {
    let cases = [
        (LanguageId::Rust, "fn main<T: Clone>(value: T) {}\n", 0, "T"),
        (
            LanguageId::TypeScript,
            "function greet<T>(value: T): Promise<void> {}\n",
            0,
            "T",
        ),
        (
            LanguageId::Go,
            "package main\ntype Box[T any] struct { Value T }\n",
            1,
            "T",
        ),
        (LanguageId::Java, "class Box<T> { T value; }\n", 0, "T"),
        (LanguageId::CSharp, "class Box<T> { T Value; }\n", 0, "T"),
        (
            LanguageId::Cpp,
            "template <typename T> class Box { T value; };\n",
            0,
            "T",
        ),
    ];
    for (language, text, row, needle) in cases {
        let spans = highlight(language, text, 0, text.lines().count());
        assert_eq!(
            name_of(&spans, text, row, needle),
            Some("type.parameter"),
            "{language:?}"
        );
    }
}

#[test]
fn highlights_members_and_parameters() {
    let java = "package demo;\nimport java.util.List;\nclass Box<T> {\n    private final List<String> items;\n    void show(int count) { items.add(String.valueOf(count)); }\n}\n";
    let spans = highlight(LanguageId::Java, java, 0, 6);
    assert_eq!(name_of(&spans, java, 0, "demo"), Some("module"));
    assert_eq!(name_of(&spans, java, 3, "items"), Some("property"));
    assert_eq!(
        name_of(&spans, java, 4, "count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, java, 4, "add"), Some("function.method"));

    let csharp = "using System;\nnamespace Demo;\nclass Box<T> {\n    public List<T> Items { get; set; }\n    private int count;\n    void Show(T item) { count = 1; }\n}\n";
    let spans = highlight(LanguageId::CSharp, csharp, 0, 7);
    assert_eq!(name_of(&spans, csharp, 0, "System"), Some("module"));
    assert_eq!(name_of(&spans, csharp, 1, "Demo"), Some("module"));
    assert_eq!(name_of(&spans, csharp, 3, "Items"), Some("property"));
    assert_eq!(name_of(&spans, csharp, 4, "count"), Some("property"));
    assert_eq!(
        name_of(&spans, csharp, 5, "item"),
        Some("variable.parameter")
    );

    let c = "#define LIMIT 10\nstatic int add(int left, int right) { return left + right; }\nenum State { Ready, Done };\n";
    let spans = highlight(LanguageId::C, c, 0, 3);
    assert_eq!(name_of(&spans, c, 0, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, c, 1, "left"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, c, 2, "Ready"), Some("constant"));
}

#[test]
fn highlights_go_calls_and_constants() {
    let text =
        "package main\nconst Limit = 10\nfunc Show(value map[string]int) { fmt.Println(value) }\n";
    let spans = highlight(LanguageId::Go, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "main"), Some("namespace"));
    assert_eq!(name_of(&spans, text, 1, "Limit"), Some("constant"));
    assert_eq!(
        name_of(&spans, text, 2, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "Println"), Some("function.method"));
}

#[test]
fn highlights_cpp_namespaces_and_methods() {
    let text = "namespace demo {\nclass Box { int value; int get(int input); };\n}\nint main() { std::vector<int> v; v.push_back(1); return 0; }\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "demo"), Some("namespace"));
    assert_eq!(name_of(&spans, text, 1, "Box"), Some("type.definition"));
    assert_eq!(
        name_of(&spans, text, 3, "push_back"),
        Some("function.method")
    );
}

#[test]
fn highlights_ruby_php_and_lua_tokens() {
    let ruby = "def show(name)\n  renamed = name\n  return renamed\nend\n";
    let spans = highlight(LanguageId::Ruby, ruby, 0, 4);
    assert_eq!(name_of(&spans, ruby, 0, "def"), Some("keyword.function"));
    assert_eq!(name_of_last(&spans, ruby, 1, "name"), Some("variable"));
    assert_eq!(name_of(&spans, ruby, 2, "return"), Some("keyword.return"));

    let php = "<?php\nuse Other\\Thing;\nclass Item extends Base {\n    public function show(int $count): void { return; }\n}\n";
    let spans = highlight(LanguageId::Php, php, 0, 5);
    assert_eq!(name_of(&spans, php, 1, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, php, 2, "Item"), Some("type.definition"));
    assert_eq!(name_of(&spans, php, 2, "Base"), Some("type"));
    assert_eq!(name_of(&spans, php, 3, "count"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, php, 3, "return"), Some("keyword.return"));

    let lua = "local M = {}\nfunction M.show(item)\n    print(item.name)\n    M.show(item)\nend\nreturn M\n";
    let spans = highlight(LanguageId::Lua, lua, 0, 6);
    assert_eq!(name_of(&spans, lua, 1, "show"), Some("function"));
    assert_eq!(name_of(&spans, lua, 2, "name"), Some("property"));
    assert_eq!(name_of(&spans, lua, 3, "show"), Some("function.call"));
}

#[test]
fn highlights_bash_builtins_and_keywords() {
    let text = "#!/bin/bash\nwork() { local name=value; echo \"$name\"; return 0; }\nfunction other { printf '%s\\n' \"$1\"; }\nsource ./lib.sh\n";
    let spans = highlight(LanguageId::Bash, text, 0, 4);
    assert_eq!(name_of(&spans, text, 1, "local"), Some("keyword.storage"));
    assert_eq!(name_of(&spans, text, 1, "echo"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 1, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 2, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 2, "printf"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 3, "source"), Some("keyword.import"));
}

#[test]
fn highlights_markdown_and_html_blocks() {
    let markdown = "# Title\n\n- item\n\n```rust\nfn main() {}\n```\n";
    let spans = highlight(LanguageId::Markdown, markdown, 0, 7);
    assert_eq!(name_of(&spans, markdown, 0, "Title"), Some("type"));
    assert_eq!(
        name_of(&spans, markdown, 2, "-"),
        Some("punctuation.special")
    );
    assert_eq!(
        name_of(&spans, markdown, 4, "```"),
        Some("punctuation.delimiter")
    );

    let html = "<div class=\"box\">&amp;</div>\n";
    let spans = highlight(LanguageId::Html, html, 0, 1);
    assert_eq!(name_of(&spans, html, 0, "&amp;"), Some("string.escape"));
}

#[test]
fn builds_a_runtime_for_every_bundled_language() {
    for language in LanguageId::ALL {
        assert!(Runtime::new(language).is_some(), "{language:?}");
    }
}
