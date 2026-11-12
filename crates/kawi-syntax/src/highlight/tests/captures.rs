use super::super::*;
use super::support::{highlight, name_at, name_of};

#[test]
fn highlights_rust_tokens() {
    let text = "// hello\nfn main() {\n    let value = 42;\n    let text = \"hi\";\n}\n";
    let spans = highlight(LanguageId::Rust, text, 0, 5);
    assert_eq!(name_at(&spans, 0, 0), Some("comment"));
    assert_eq!(name_at(&spans, 1, 0), Some("keyword.function"));
    assert_eq!(name_at(&spans, 1, 3), Some("function"));
    assert_eq!(name_at(&spans, 2, 4), Some("keyword"));
    assert_eq!(name_at(&spans, 2, 16), Some("number"));
    assert_eq!(name_at(&spans, 3, 15), Some("string"));
}

#[test]
fn keeps_documentation_and_parameter_captures() {
    let rust = "/// docs\nfn main(value: u8) {}\n";
    let spans = highlight(LanguageId::Rust, rust, 0, 2);
    assert_eq!(name_at(&spans, 0, 0), Some("comment.documentation"));
    assert_eq!(name_at(&spans, 1, 9), Some("variable.parameter"));

    let python = "def greet(name: str):\n    return name\n";
    let spans = highlight(LanguageId::Python, python, 0, 2);
    assert_eq!(name_at(&spans, 0, 10), Some("variable.parameter"));
}

#[test]
fn highlights_toml_tokens() {
    let text = "# comment\nname = \"kawi\"\n\"version\" = 1\nenabled = true\n";
    let spans = highlight(LanguageId::Toml, text, 0, 4);
    assert_eq!(name_at(&spans, 0, 0), Some("comment"));
    assert_eq!(name_at(&spans, 1, 7), Some("string"));
    assert_eq!(name_at(&spans, 2, 1), Some("property"));
    assert_eq!(name_at(&spans, 2, 12), Some("number"));
    assert_eq!(name_at(&spans, 3, 10), Some("boolean"));
}

#[test]
fn highlights_typescript_tokens() {
    let text =
        "const x: string = \"hi\";\nfunction greet(name: string) { return name; }\n// tail\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 3);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword"));
    assert_eq!(name_at(&spans, 0, 18), Some("string"));
    assert_eq!(name_at(&spans, 1, 0), Some("keyword.function"));
    assert_eq!(name_at(&spans, 1, 9), Some("function"));
    assert_eq!(name_at(&spans, 2, 0), Some("comment"));
}

#[test]
fn highlights_python_tokens() {
    let text = "def greet(name):\n    # comment\n    return \"hello\"\n";
    let spans = highlight(LanguageId::Python, text, 0, 3);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword.function"));
    assert_eq!(name_at(&spans, 0, 4), Some("function"));
    assert_eq!(name_at(&spans, 1, 4), Some("comment"));
    assert_eq!(name_at(&spans, 2, 11), Some("string"));
}

#[test]
fn highlights_rust_imports_and_returns() {
    let text = "use std::fmt;\nfn render() -> u8 { return 1; }\n";
    let spans = highlight(LanguageId::Rust, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "return"), Some("keyword.return"));
}

#[test]
fn highlights_builtin_and_macro_captures() {
    let rust = "fn show(ok: bool) { println!(\"hi\"); let value = true; }\n";
    let spans = highlight(LanguageId::Rust, rust, 0, 1);
    assert_eq!(name_of(&spans, rust, 0, "println"), Some("function.macro"));
    assert_eq!(name_of(&spans, rust, 0, "true"), Some("boolean"));

    let python = "class Item:\n    def show(self, name):\n        print(name)\n";
    let spans = highlight(LanguageId::Python, python, 0, 3);
    assert_eq!(name_of(&spans, python, 1, "self"), Some("variable.builtin"));
    assert_eq!(
        name_of(&spans, python, 2, "print"),
        Some("function.builtin")
    );
}

#[test]
fn highlights_shorthand_members() {
    let rust = "fn build(input: u8) { let item = Item { field }; }\n";
    let spans = highlight(LanguageId::Rust, rust, 0, 1);
    assert_eq!(
        name_of(&spans, rust, 0, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, rust, 0, "field"), Some("variable.member"));

    let javascript = "const source = 1; const item = { field };\n";
    let spans = highlight(LanguageId::JavaScript, javascript, 0, 1);
    assert_eq!(
        name_of(&spans, javascript, 0, "field"),
        Some("variable.member")
    );
}

#[test]
fn highlights_python_imports_and_builtin_types() {
    let text = "from pathlib import Path\ndef greet(name: str) -> str:\n    return name\n";
    let spans = highlight(LanguageId::Python, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "from"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "return"), Some("keyword.return"));
}

#[test]
fn highlights_json_tokens() {
    let text = "{\"name\": \"kawi\", \"count\": 2}\n";
    let spans = highlight(LanguageId::Json, text, 0, 1);
    assert_eq!(name_at(&spans, 0, 1), Some("property"));
    assert_eq!(name_at(&spans, 0, 9), Some("string"));
    assert_eq!(name_at(&spans, 0, 26), Some("number"));
}

#[test]
fn highlights_bash_tokens() {
    let text = "#!/bin/bash\n# comment\necho \"hi\"\n";
    let spans = highlight(LanguageId::Bash, text, 0, 3);
    assert_eq!(name_at(&spans, 1, 0), Some("comment"));
    assert_eq!(name_at(&spans, 2, 5), Some("string"));
}

#[test]
fn builds_a_runtime_for_every_language() {
    for language in [
        LanguageId::Rust,
        LanguageId::Toml,
        LanguageId::TypeScript,
        LanguageId::Tsx,
        LanguageId::JavaScript,
        LanguageId::Jsx,
        LanguageId::Python,
        LanguageId::Json,
        LanguageId::Bash,
    ] {
        assert!(Runtime::new(language).is_some(), "{language:?}");
    }
}

#[test]
fn highlights_javascript_tokens() {
    let text = "const value = \"hi\"; // tail\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 1);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword"));
    assert_eq!(name_at(&spans, 0, 14), Some("string"));
    assert_eq!(name_at(&spans, 0, 20), Some("comment"));
}

#[test]
fn highlights_javascript_parameters() {
    let text = "function greet(name) { return name; }\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 1);
    assert_eq!(name_at(&spans, 0, 15), Some("variable.parameter"));
}

#[test]
fn highlights_script_keywords_and_destructured_parameters() {
    let javascript = "import value from \"mod\";\nfunction greet({ name, age }) { return name; }\n";
    let spans = highlight(LanguageId::JavaScript, javascript, 0, 2);
    assert_eq!(
        name_of(&spans, javascript, 0, "import"),
        Some("keyword.import")
    );
    assert_eq!(
        name_of(&spans, javascript, 0, "from"),
        Some("keyword.import")
    );
    assert_eq!(
        name_of(&spans, javascript, 1, "name"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, javascript, 1, "age"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, javascript, 1, "return"),
        Some("keyword.return")
    );

    let typescript = "import { value } from \"mod\";\nfunction greet({ name, age }: Person): number { return 1; }\n";
    let spans = highlight(LanguageId::TypeScript, typescript, 0, 2);
    assert_eq!(
        name_of(&spans, typescript, 0, "import"),
        Some("keyword.import")
    );
    assert_eq!(
        name_of(&spans, typescript, 0, "from"),
        Some("keyword.import")
    );
    assert_eq!(
        name_of(&spans, typescript, 1, "name"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, typescript, 1, "age"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, typescript, 1, "Person"), Some("type"));
    assert_eq!(
        name_of(&spans, typescript, 1, "number"),
        Some("type.builtin")
    );
    assert_eq!(
        name_of(&spans, typescript, 1, "return"),
        Some("keyword.return")
    );
}

#[test]
fn highlights_jsx_component_tags_and_attributes() {
    let text = "const view = <Widget prop={value} />;\n";
    let spans = highlight(LanguageId::Jsx, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "Widget"), Some("tag"));
    assert_eq!(name_of(&spans, text, 0, "prop"), Some("tag.attribute"));
}

#[test]
fn highlights_tsx_imports_and_component_tags() {
    let text = "import { value } from \"mod\";\nconst view = <Widget prop={value} />;\n";
    let spans = highlight(LanguageId::Tsx, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "Widget"), Some("tag"));
    assert_eq!(name_of(&spans, text, 1, "prop"), Some("tag.attribute"));
}

#[test]
fn highlights_bash_variables() {
    let text = "name=value\necho \"$name\"\n";
    let spans = highlight(LanguageId::Bash, text, 0, 2);
    assert_eq!(name_of(&spans, text, 1, "name"), Some("variable"));
}

#[test]
fn highlights_jsx_tokens() {
    let text = "const view = <div className=\"x\">hi</div>;\n";
    let spans = highlight(LanguageId::Jsx, text, 0, 1);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword"));
    assert_eq!(name_at(&spans, 0, 18), Some("tag.attribute"));
    assert_eq!(name_at(&spans, 0, 30), Some("string"));
    assert_eq!(name_at(&spans, 0, 14), Some("tag"));
}

#[test]
fn highlights_tsx_tokens() {
    let text = "const view: string = <div className=\"x\">hi</div>;\n";
    let spans = highlight(LanguageId::Tsx, text, 0, 1);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword"));
    assert_eq!(name_at(&spans, 0, 27), Some("tag.attribute"));
    assert_eq!(name_at(&spans, 0, 36), Some("string"));
    assert_eq!(name_at(&spans, 0, 22), Some("tag"));
}
