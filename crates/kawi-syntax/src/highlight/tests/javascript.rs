use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_javascript_keywords_builtins_and_calls() {
    let text = "import value from \"mod\";\nconst pattern = /ab+c/gi;\nasync function load(input) {\n    if (input) { throw new Error(\"bad\"); }\n    for (const item of []) { console.log(item); }\n    return Intl.NumberFormat(\"en\").format(item);\n}\nconst ok = true;\nconst empty = null;\nconst root = this;\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "from"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "ab+c"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 1, "gi"), Some("character.special"));
    assert_eq!(name_of(&spans, text, 2, "async"), Some("keyword.coroutine"));
    assert_eq!(
        name_of(&spans, text, 2, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 2, "load"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 2, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 3, "throw"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 4, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "of"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 4, "console"),
        Some("variable.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 4, "log"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 5, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 5, "Intl"), Some("module.builtin"));
    assert_eq!(
        name_of(&spans, text, 5, "NumberFormat"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 7, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 8, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 9, "this"), Some("variable.builtin"));
}

#[test]
fn highlights_javascript_functions_methods_and_members() {
    let text = "function greet(name) { return name; }\nconst build = (left, right) => left + right;\nconst id = value => value;\nclass Box {\n    constructor() {}\n    show() { this.value = other.label; }\n}\nconst item = { label, render() { return 1; } };\nconst box = new Box();\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "greet"), Some("function"));
    assert_eq!(name_of(&spans, text, 0, "name"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "build"), Some("function"));
    assert_eq!(name_of(&spans, text, 1, "left"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 1, "right"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "id"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 2, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "constructor"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 5, "show"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 5, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 5, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 5, "label"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 7, "label"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 7, "render"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 8, "Box"), Some("constructor"));
}

#[test]
fn highlights_javascript_optional_chain_literals_and_escapes() {
    let text = "const user = { profile: null };\nconst name = user?.profile?.name;\nconst text = `hi ${name}`;\nconst escaped = \"a\\nb\";\nconst pattern = /a+/gi;\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 5);
    let chain = text
        .lines()
        .nth(1)
        .expect("row")
        .find("?.")
        .expect("optional chain");
    assert_eq!(name_at(&spans, 1, chain), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 1, "profile"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 2, "${"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 2, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "a\\nb"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 4, "a+"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 4, "gi"), Some("character.special"));
}
