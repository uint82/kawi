use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_go_packages_types_and_generics() {
    let text = "// Package demo exercises type declarations.\npackage demo\n\n// ID names a user identifier.\ntype ID string\n\ntype Box[T any] struct {\n    Value T\n    inner []T\n}\n\ntype Shape interface {\n    Draw() string\n}\n\ntype Number interface {\n    ~int | ~float64\n}\n\ntype Alias = Box[int]\n";
    let spans = highlight(LanguageId::Go, text, 0, 20);
    assert_eq!(
        name_of(&spans, text, 0, "Package"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 1, "package"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "demo"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 3, "ID"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 4, "type"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 4, "ID"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 4, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 6, "struct"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 6, "Box"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 6, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 6, "any"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 7, "Value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 8, "inner"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 11, "interface"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 11, "Shape"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 12, "Draw"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 12, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 15, "Number"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 16, "~"), Some("operator"));
    assert_eq!(name_of(&spans, text, 16, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 16, "|"), Some("operator"));
    assert_eq!(name_of(&spans, text, 16, "float64"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 19, "Alias"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 19, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 19, "int"), Some("type.builtin"));
}

#[test]
fn highlights_go_functions_methods_and_calls() {
    let text = "package demo\n\nfunc NewBox(value int) *Box {\n    return &Box{value: value}\n}\n\nfunc MakeBox(value int) *Box {\n    return NewBox(value)\n}\n\nfunc (b *Box) Show() string {\n    return format(b)\n}\n\nfunc format(value *Box) string { return \"\" }\n\nfunc join(left, right string, values ...int) string {\n    box := NewBox(1)\n    items := make([]string, 0)\n    items = append(items, left, right)\n    _ = box.Show()\n    _ = MakeBox(2)\n    _ = len(items)\n    return format(&Box{value: 1}) + left\n}\n";
    let spans = highlight(LanguageId::Go, text, 0, 24);
    assert_eq!(name_of(&spans, text, 2, "func"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 2, "NewBox"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 2, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 3, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 6, "MakeBox"), Some("function"));
    assert_eq!(name_of(&spans, text, 7, "NewBox"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 10, "Show"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 10, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 11, "format"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 14, "format"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 14, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 16, "join"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 16, "left"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 16, "right"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 16, "values"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 16, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 16, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 17, "NewBox"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 18, "make"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 18, "string"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 19, "append"),
        Some("function.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 20, "Show"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 21, "MakeBox"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 22, "len"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 23, "format"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 23, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 23, "value"), Some("variable.member"));
}

#[test]
fn highlights_go_members_labels_and_composite_literals() {
    let text = "package demo\n\ntype Config struct {\n    Name string\n    values []int\n}\n\nfunc (c Config) Apply(input Config) {\n    c.Name = input.Name\nlabel:\n    for index, value := range c.values {\n        if index > 0 {\n            break label\n        }\n        _ = value\n    }\n}\n\nfunc build() Config {\n    return Config{Name: \"x\", values: []int{1, 2}}\n}\n";
    let spans = highlight(LanguageId::Go, text, 0, 20);
    assert_eq!(name_of(&spans, text, 2, "Config"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 3, "Name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 4, "values"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 7, "Apply"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 7, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 8, "Name"), Some("property"));
    assert_eq!(name_of(&spans, text, 9, "label"), Some("label"));
    assert_eq!(name_of(&spans, text, 10, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 10, "index"), Some("variable"));
    assert_eq!(name_of(&spans, text, 10, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 10, "range"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 12, "break"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 12, "label"), Some("label"));
    assert_eq!(name_of(&spans, text, 18, "build"), Some("function"));
    assert_eq!(name_of(&spans, text, 19, "Config"), Some("type"));
    assert_eq!(name_of(&spans, text, 19, "Name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 19, "values"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 19, "\"x\""), Some("string"));
    assert_eq!(name_of(&spans, text, 19, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 19, "1"), Some("number"));
}

#[test]
fn highlights_go_keywords_and_operators() {
    let text = "package demo\n\nimport \"fmt\"\n\nfunc run(ch chan int) {\n    defer fmt.Println(\"done\")\n    go worker()\n    for index := range []int{1, 2} {\n        if index < 0 || index > 10 {\n            continue\n        } else {\n            switch index {\n            case 0:\n                fallthrough\n            default:\n                goto finish\n            }\n        }\n    }\nfinish:\n    select {\n    case value := <-ch:\n        _ = value\n    default:\n    }\n    var total int\n    total += len([]int{1})\n    _ = total\n}\n\nfunc worker() {}\n";
    let spans = highlight(LanguageId::Go, text, 0, 31);
    assert_eq!(name_of(&spans, text, 0, "package"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 2, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 2, "\"fmt\""), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "func"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 4, "run"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "ch"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 4, "chan"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 4, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "defer"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 5, "Println"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 6, "go"), Some("keyword.coroutine"));
    assert_eq!(name_of(&spans, text, 6, "worker"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 7, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 7, "index"), Some("variable"));
    assert_eq!(name_of(&spans, text, 7, ":="), Some("operator"));
    assert_eq!(name_of(&spans, text, 7, "range"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 8, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 8, "<"), Some("operator"));
    assert_eq!(name_of(&spans, text, 8, "||"), Some("operator"));
    assert_eq!(name_of(&spans, text, 8, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 9, "continue"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 10, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 11, "switch"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 12, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 13, "fallthrough"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 14, "default"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 15, "goto"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 19, "finish"), Some("label"));
    assert_eq!(name_of(&spans, text, 20, "select"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 21, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 21, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 21, "<-"), Some("operator"));
    assert_eq!(name_of(&spans, text, 23, "default"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 25, "var"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 25, "total"), Some("variable"));
    assert_eq!(name_of(&spans, text, 26, "+="), Some("operator"));
    assert_eq!(name_of(&spans, text, 26, "len"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 30, "worker"), Some("function"));
}

#[test]
fn highlights_go_literals_and_documentation() {
    let text = "// Package demo is a literal sample.\npackage demo\n\n// Limit is the maximum.\nconst Limit = 10\n\n// Ratio scales the sample.\nvar Ratio = 1.5\n\n// values returns literal samples.\nfunc values() {\n    ratio := 1.5\n    imaginary := 2i\n    flag := true\n    missing := nil\n    index := iota\n    text := \"hello\\n\"\n    raw := `raw\ntext`\n    letter := 'x'\n    // local note\n}\n";
    let spans = highlight(LanguageId::Go, text, 0, 22);
    assert_eq!(
        name_of(&spans, text, 0, "Package"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 1, "demo"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 3, "Limit"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 4, "const"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 4, "Limit"), Some("constant"));
    assert_eq!(name_of(&spans, text, 4, "10"), Some("number"));
    assert_eq!(
        name_of(&spans, text, 6, "Ratio"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 7, "var"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "Ratio"), Some("variable"));
    assert_eq!(name_of(&spans, text, 7, "1.5"), Some("number.float"));
    assert_eq!(
        name_of(&spans, text, 9, "values"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 10, "func"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 10, "values"), Some("function"));
    assert_eq!(name_of(&spans, text, 11, "ratio"), Some("variable"));
    assert_eq!(name_of(&spans, text, 11, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 12, "imaginary"), Some("variable"));
    assert_eq!(name_of(&spans, text, 12, "2i"), Some("number"));
    assert_eq!(name_of(&spans, text, 13, "flag"), Some("variable"));
    assert_eq!(name_of(&spans, text, 13, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 14, "missing"), Some("variable"));
    assert_eq!(name_of(&spans, text, 14, "nil"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 15, "index"), Some("variable"));
    assert_eq!(name_of(&spans, text, 15, "iota"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 16, "text"), Some("variable"));
    assert_eq!(name_of(&spans, text, 16, "hello"), Some("string"));
    assert_eq!(name_of(&spans, text, 16, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 18, "text"), Some("string"));
    assert_eq!(name_of(&spans, text, 19, "letter"), Some("variable"));
    assert_eq!(name_of(&spans, text, 19, "'x'"), Some("character"));
    assert_eq!(name_of(&spans, text, 20, "local"), Some("comment"));
}

#[test]
fn highlights_go_regexp_patterns() {
    let text = "package demo\n\nimport (\n    \"fmt\"\n    \"regexp\"\n    \"strings\"\n)\n\nfunc match(text string) bool {\n    pattern := regexp.MustCompile(`^[a-z]+$`)\n    direct := regexp.MatchString(\"^abc$\", text)\n    upper := strings.ToUpper(text)\n    fmt.Println(upper)\n    return pattern.MatchString(text) || direct\n}\n";
    let spans = highlight(LanguageId::Go, text, 0, 15);
    assert_eq!(name_of(&spans, text, 8, "match"), Some("function"));
    assert_eq!(name_of(&spans, text, 8, "text"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 8, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 8, "bool"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 9, "MustCompile"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 9, "^[a-z]+$"), Some("string.regexp"));
    assert_eq!(
        name_of(&spans, text, 10, "MatchString"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 10, "^abc$"), Some("string.regexp"));
    assert_eq!(
        name_of(&spans, text, 11, "ToUpper"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 12, "Println"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 13, "MatchString"),
        Some("function.method.call")
    );
}

#[test]
fn highlights_go_builtin_functions() {
    let text = "package demo\n\nfunc use(values []int) {\n    clear(values)\n    _ = max(1, 2)\n    _ = min(1, 2)\n    _ = cap(values)\n    _ = complex(1, 2)\n    _ = real(1 + 2i)\n    _ = imag(2i)\n    close(make(chan int))\n    delete(map[string]int{}, \"x\")\n    copy(values, values)\n    _ = new(int)\n}\n";
    let spans = highlight(LanguageId::Go, text, 0, 15);
    assert_eq!(name_of(&spans, text, 3, "clear"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 4, "max"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 5, "min"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 6, "cap"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 7, "complex"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 8, "real"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 9, "imag"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 10, "close"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 10, "make"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 10, "chan"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 11, "delete"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 11, "map"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 11, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 12, "copy"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 13, "new"), Some("function.builtin"));
}
