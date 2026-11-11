use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_typescript_keywords_builtins_and_calls() {
    let text = "import type { Foo } from \"./foo\";\nconst MAX_SIZE = 42;\nconst value = new Map<string, number>();\nasync function load<T>(input: T): Promise<T> {\n    if (input) { throw new Error(\"bad\"); }\n    for (const item of []) { console.log(item); }\n    return input;\n}\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 8);
    assert_eq!(name_of(&spans, text, 0, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "type"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "Foo"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "from"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "MAX_SIZE"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "Map"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 2, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "async"), Some("keyword.coroutine"));
    assert_eq!(
        name_of(&spans, text, 3, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 3, "load"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "T"), Some("type.parameter"));
    assert_eq!(
        name_of(&spans, text, 3, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "Promise"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 4, "throw"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 5, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 5, "of"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 5, "console"),
        Some("variable.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 5, "log"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 6, "return"), Some("keyword.return"));
}

#[test]
fn highlights_typescript_type_syntax() {
    let text = "interface Shape {\n    readonly kind?: string;\n    area(): number;\n}\ntype Mapped<T> = { readonly [K in keyof T]?: T[K] };\nfunction check(value: unknown): value is string { return typeof value === \"string\"; }\nconst converted = value as string;\nconst nonNull = value!;\nconst id = value => value;\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "interface"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "Shape"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 1, "readonly"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 1, "kind"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 1, "?"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 1, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "area"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 4, "type"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 4, "Mapped"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 4, "in"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 4, "keyof"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 5, "is"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 5, "typeof"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 6, "as"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 7, "!"), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 8, "value"),
        Some("variable.parameter")
    );
}

#[test]
fn highlights_typescript_literals_and_regex() {
    let text = "const pattern = /ab+c/gi;\nconst text = `hi ${name}`;\nconst ok = true;\nconst empty = null;\nrequire(\"./mod\");\nconsole.log(pattern);\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "ab+c"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 0, "gi"), Some("character.special"));
    assert_eq!(name_of(&spans, text, 0, "/"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 1, "${"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 1, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 3, "null"), Some("constant.builtin"));
    assert_eq!(
        name_of(&spans, text, 4, "require"),
        Some("function.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 5, "console"),
        Some("variable.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 5, "log"),
        Some("function.method.call")
    );
}

#[test]
fn highlights_tsx_components_attributes_and_expressions() {
    let text = "const view = <Widget title=\"x\" href=\"/home\">{value}</Widget>;\nconst dot = <My.Component data-id=\"y\" />;\nconst plain = <div className=\"box\">text</div>;\n";
    let spans = highlight(LanguageId::Tsx, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "Widget"), Some("tag"));
    assert_eq!(name_of(&spans, text, 0, "title"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 0, "href"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 0, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 1, "My"), Some("tag"));
    assert_eq!(name_of(&spans, text, 1, "Component"), Some("tag"));
    assert_eq!(name_of(&spans, text, 1, "data-id"), Some("tag.attribute"));
    assert_eq!(name_of(&spans, text, 2, "div"), Some("tag"));
    assert_eq!(name_of(&spans, text, 2, "className"), Some("tag.attribute"));
    let bracket = text.lines().next().expect("row").find('<').expect("<");
    assert_eq!(name_at(&spans, 0, bracket), Some("tag.delimiter"));
    let url = text
        .lines()
        .next()
        .expect("row")
        .find("/home")
        .expect("url");
    assert_eq!(name_at(&spans, 0, url), Some("string.special.url"));
}

#[test]
fn highlights_typescript_type_argument_brackets() {
    let text = "const value = new Map<string, number>();\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 1);
    let open = text.find('<').expect("<");
    assert_eq!(name_at(&spans, 0, open), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, "Map"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 0, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "number"), Some("type.builtin"));
}
