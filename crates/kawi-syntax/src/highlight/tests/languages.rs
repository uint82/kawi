use super::support::{highlight, name_of};
use super::*;

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
fn builds_a_runtime_for_every_bundled_language() {
    for language in LanguageId::ALL {
        assert!(Runtime::new(language).is_some(), "{language:?}");
    }
}
