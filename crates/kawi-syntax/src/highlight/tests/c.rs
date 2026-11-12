use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_c_preprocessor_directives_and_macros() {
    let text = "#include <stdio.h>\n#include \"local.h\"\n\n#define LIMIT 10\n#define MAX(a, b) ((a) > (b) ? (a) : (b))\n#undef LIMIT\n\n#if defined(LIMIT) && !defined(FALLBACK)\n#define FALLBACK 0\n#elifdef LIMIT\n#define FALLBACK 1\n#endif\n\n#ifndef HEADER_H\n#define HEADER_H\n#endif\n";
    let spans = highlight(LanguageId::C, text, 0, 15);
    assert_eq!(name_of(&spans, text, 0, "#include"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "<stdio.h>"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "local.h"), Some("string"));
    assert_eq!(
        name_of(&spans, text, 3, "#define"),
        Some("keyword.directive.define")
    );
    assert_eq!(name_of(&spans, text, 3, "LIMIT"), Some("constant.macro"));
    assert_eq!(name_of(&spans, text, 4, "MAX"), Some("function.macro"));
    assert_eq!(name_of(&spans, text, 4, "a"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 5, "#undef"),
        Some("keyword.directive")
    );
    assert_eq!(name_of(&spans, text, 5, "LIMIT"), Some("constant.macro"));
    assert_eq!(name_of(&spans, text, 7, "#if"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 7, "defined"), Some("function.macro"));
    assert_eq!(name_of(&spans, text, 7, "FALLBACK"), Some("constant.macro"));
    assert_eq!(
        name_of(&spans, text, 9, "#elifdef"),
        Some("keyword.directive")
    );
    assert_eq!(
        name_of(&spans, text, 13, "#ifndef"),
        Some("keyword.directive")
    );
    assert_eq!(
        name_of(&spans, text, 14, "HEADER_H"),
        Some("constant.macro")
    );
}

#[test]
fn highlights_c_types_and_aggregate_members() {
    let text = "typedef struct Point {\n    int x;\n    int *y;\n    int values[4];\n} Point;\n\nunion Value {\n    long integer;\n    double real;\n};\n\nenum Color {\n    COLOR_RED,\n    COLOR_GREEN = 2,\n};\n\ntypedef unsigned long size_type;\n\nstatic const int LIMIT = 10;\n";
    let spans = highlight(LanguageId::C, text, 0, 18);
    assert_eq!(name_of(&spans, text, 0, "typedef"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "Point"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "x"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "y"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "values"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "Point"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 6, "union"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 6, "Value"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "long"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 7, "integer"), Some("property"));
    assert_eq!(name_of(&spans, text, 11, "enum"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 11, "Color"), Some("type"));
    assert_eq!(name_of(&spans, text, 12, "COLOR_RED"), Some("constant"));
    assert_eq!(name_of(&spans, text, 13, "COLOR_GREEN"), Some("constant"));
    assert_eq!(
        name_of(&spans, text, 16, "size_type"),
        Some("type.definition")
    );
    assert_eq!(
        name_of(&spans, text, 18, "static"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 18, "const"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 18, "LIMIT"), Some("constant"));
}

#[test]
fn highlights_c_functions_calls_and_labels() {
    let text = "static int compute(const char *input, int values[], size_t count);\n\nint compute(const char *input, int values[], size_t count) {\n    int total = 0;\n    for (int i = 0; i < count; i++) {\n        total += values[i];\n    }\n    goto done;\ndone:\n    return total;\n}\n\nvoid run(struct Point *point) {\n    compute(\"x\", NULL, 0);\n    point->callback(1);\n    point->field = 2;\n    __builtin_trap();\n}\n";
    let spans = highlight(LanguageId::C, text, 0, 17);
    assert_eq!(name_of(&spans, text, 0, "compute"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 0, "input"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 0, "values"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 0, "size_t"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 0, "count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "compute"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "total"), Some("variable"));
    assert_eq!(name_of(&spans, text, 4, "for"), Some("keyword.repeat"));
    assert_eq!(name_at(&spans, 4, 13), Some("variable"));
    assert_eq!(name_of(&spans, text, 5, "values"), Some("variable"));
    assert_eq!(name_of(&spans, text, 7, "goto"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "done"), Some("label"));
    assert_eq!(name_of(&spans, text, 8, "done"), Some("label"));
    assert_eq!(name_of(&spans, text, 9, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 12, "run"), Some("function"));
    assert_eq!(name_of(&spans, text, 12, "Point"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 12, "point"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 13, "compute"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 13, "NULL"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 14, "callback"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 15, "field"), Some("property"));
    assert_eq!(
        name_of(&spans, text, 16, "__builtin_trap"),
        Some("function.builtin")
    );
}

#[test]
fn highlights_c_literals_operators_and_comments() {
    let text = "/* block comment */\n/** Documented function. */\nint choose(int value) {\n    char letter = 'a';\n    const char *text = \"hi\\n\";\n    double ratio = 1.5;\n    int mask = 0xFFu;\n    int result = value > 0 ? value : -value;\n    bool ok = true;\n    void *nothing = NULL;\n    return ok ? result : 0;\n}\n";
    let spans = highlight(LanguageId::C, text, 0, 11);
    assert_eq!(
        name_of(&spans, text, 0, "/* block comment */"),
        Some("comment")
    );
    assert_eq!(
        name_of(&spans, text, 1, "/** Documented function. */"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 2, "choose"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "char"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "letter"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "'a'"), Some("character"));
    assert_eq!(name_of(&spans, text, 4, "const"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 4, "text"), Some("variable"));
    assert_eq!(name_of(&spans, text, 4, "hi"), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 5, "ratio"), Some("variable"));
    assert_eq!(name_of(&spans, text, 5, "1.5"), Some("number"));
    assert_eq!(name_of(&spans, text, 6, "mask"), Some("variable"));
    assert_eq!(name_of(&spans, text, 6, "0xFFu"), Some("number"));
    assert_eq!(name_of(&spans, text, 7, ">"), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 7, "?"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(
        name_of(&spans, text, 7, ":"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(name_of(&spans, text, 8, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 9, "NULL"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 10, "return"), Some("keyword.return"));
}

#[test]
fn highlights_c_designators_attributes_and_switch_cases() {
    let text = "struct Config {\n    int width;\n    int *height;\n    int values[4];\n};\n\nvoid init(void) {\n    struct Config config = {\n        .width = 1,\n        .height = NULL,\n    };\n    switch (config.width) {\n        case CONFIG_WIDTH:\n            break;\n        default:\n            break;\n    }\n    __attribute__((unused)) int local = 0;\n}\n";
    let spans = highlight(LanguageId::C, text, 0, 18);
    assert_eq!(name_of(&spans, text, 0, "Config"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "width"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "height"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "values"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, "config"), Some("variable"));
    assert_eq!(name_of(&spans, text, 8, "width"), Some("property"));
    assert_eq!(name_of(&spans, text, 9, "height"), Some("property"));
    assert_eq!(name_of(&spans, text, 9, "NULL"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 11, "config"), Some("variable"));
    assert_eq!(name_of(&spans, text, 11, "width"), Some("property"));
    assert_eq!(
        name_of(&spans, text, 12, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 12, "CONFIG_WIDTH"), Some("constant"));
    assert_eq!(name_of(&spans, text, 13, "break"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 14, "default"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 17, "__attribute__"),
        Some("attribute")
    );
    assert_eq!(
        name_of(&spans, text, 17, "unused"),
        Some("variable.builtin")
    );
    assert_eq!(name_of(&spans, text, 17, "local"), Some("variable"));
}
