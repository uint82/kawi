use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_cpp_namespaces_and_type_definitions() {
    let text = "namespace demo {\nnamespace Util {\nclass Widget {\npublic:\n    int value;\n};\nstruct Point { int x; };\nunion Value { int integer; double real; };\nenum class Color { Red, Green };\ntemplate <typename T>\nconcept Numeric = requires(T value) { value + value; };\ntemplate <typename T>\nusing Box = std::vector<T>;\n}\n}\nusing namespace demo;\nstatic_assert(sizeof(int) >= 4, \"int too small\");\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 17);
    assert_eq!(name_of(&spans, text, 0, "demo"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "Util"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "Widget"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 4, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 6, "Point"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 6, "x"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 7, "Value"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 8, "Color"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 9, "T"), Some("type.parameter"));
    assert_eq!(
        name_of(&spans, text, 10, "Numeric"),
        Some("type.definition")
    );
    assert_eq!(name_of(&spans, text, 12, "Box"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 12, "std"), Some("module"));
    assert_eq!(name_of(&spans, text, 15, "demo"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 16, "static_assert"),
        Some("function.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 16, "sizeof"),
        Some("keyword.operator")
    );
}

#[test]
fn highlights_cpp_functions_methods_and_calls() {
    let text = "class Widget {\npublic:\n    Widget(int value);\n    ~Widget();\n    int value() const;\n    void set_value(int new_value);\n    void m_cached();\n    void draw() const;\n};\n\nint Widget::value() const { return value_; }\n\nvoid Widget::set_value(int new_value) { value_ = new_value; }\n\nint add(int left, int right = 0);\n\nint use() {\n    auto widget = Widget(1);\n    widget.set_value(2);\n    auto result = add(1, 2);\n    auto shifted = std::move(result);\n    return result;\n}\n\nWidget::Widget(int value) : value_(value) {}\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 25);
    assert_eq!(name_of(&spans, text, 2, "Widget"), Some("constructor"));
    assert_eq!(
        name_of(&spans, text, 2, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "Widget"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 4, "value"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 5, "set_value"),
        Some("function.method")
    );
    assert_eq!(
        name_of(&spans, text, 5, "new_value"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 6, "m_cached"),
        Some("function.method")
    );
    assert_eq!(name_of(&spans, text, 7, "draw"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 10, "value"), Some("function"));
    assert_eq!(name_of(&spans, text, 12, "set_value"), Some("function"));
    assert_eq!(name_of(&spans, text, 14, "add"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 14, "right"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 14, "0"), Some("number"));
    assert_eq!(name_of(&spans, text, 17, "Widget"), Some("constructor"));
    assert_eq!(
        name_of(&spans, text, 18, "set_value"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 19, "add"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 20, "move"), Some("function.call"));
    assert_eq!(name_at(&spans, 24, 8), Some("constructor"));
    assert_eq!(
        name_of(&spans, text, 24, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 24, "value_"), Some("property"));
}

#[test]
fn highlights_cpp_members_and_parameters() {
    let text = "class Config {\npublic:\n    void apply(const Config &other, int values[], int &count, int limit = 4);\n    int m_cached;\n    std::string name;\n};\n\nvoid apply(const Config &other, int values[], int &count, int limit) {\n    this->m_cached = limit;\n    name = \"x\";\n}\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 10);
    assert_eq!(name_of(&spans, text, 2, "apply"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 2, "other"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 2, "values"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 2, "count"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 2, "limit"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 3, "m_cached"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 4, "name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 8, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 8, "m_cached"), Some("property"));
    assert_eq!(name_of(&spans, text, 9, "name"), Some("variable"));
}

#[test]
fn highlights_cpp_templates_and_named_casts() {
    let text = "template <typename T>\nT cast_value(T input) {\n    auto shifted = std::move(input);\n    auto number = static_cast<long>(input);\n    auto kind = typeid(T);\n    return number;\n}\n\nint deep() {\n    return demo::util::math::ops::add(1, 2);\n}\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 11);
    assert_eq!(name_of(&spans, text, 0, "<"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 1, "cast_value"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 1, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "auto"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "move"), Some("function.call"));
    assert_eq!(
        name_of(&spans, text, 3, "static_cast"),
        Some("keyword.operator")
    );
    assert_eq!(name_of(&spans, text, 3, "long"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 4, "typeid"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 9, "add"), Some("function.call"));
}

#[test]
fn highlights_cpp_keywords_exceptions_and_literals() {
    let text = "struct Base {\n    virtual void draw() = 0;\n};\n\nstruct Derived final : Base {\n    void draw() override;\n};\n\nvoid run() noexcept {\n    try {\n        throw 1;\n    } catch (...) {\n    }\n    auto text = R\"(raw)\";\n    auto amount = 1.5f;\n    auto compare = 1 <=> 2;\n}\n\nint task() {\n    co_return 1;\n}\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 21);
    assert_eq!(
        name_of(&spans, text, 1, "virtual"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 4, "final"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 5, "override"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 8, "noexcept"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 9, "try"), Some("keyword.exception"));
    assert_eq!(
        name_of(&spans, text, 10, "throw"),
        Some("keyword.exception")
    );
    assert_eq!(
        name_of(&spans, text, 11, "catch"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 13, "R\"(raw)\""), Some("string"));
    assert_eq!(name_of(&spans, text, 14, "1.5f"), Some("number"));
    assert_eq!(name_of(&spans, text, 15, "<=>"), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 19, "co_return"),
        Some("keyword.coroutine")
    );
}

#[test]
fn highlights_cpp_pointer_and_reference_default_parameters() {
    let text = "struct Config {};\nvoid apply(int *value = nullptr, int &ref = *value, int **pointer = nullptr, int *&link = pointer, const Config &other = Config());\n";
    let spans = highlight(LanguageId::Cpp, text, 0, 2);
    assert_eq!(
        name_of(&spans, text, 1, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "ref"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 1, "pointer"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "link"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 1, "other"),
        Some("variable.parameter")
    );
}
