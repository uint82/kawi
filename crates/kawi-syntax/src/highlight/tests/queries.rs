use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_javascript_default_and_rest_parameters() {
    let text = "function greet(name = \"a\", { age, city = \"x\", ...extra }, [first, ...rest]) { return name; }\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 1);
    for parameter in ["name", "age", "city", "extra", "first", "rest"] {
        assert_eq!(
            name_of(&spans, text, 0, parameter),
            Some("variable.parameter"),
            "{parameter}"
        );
    }
}

#[test]
fn highlights_javascript_destructured_default_parameters() {
    let text = "const view = ({ name } = {}, [title] = []) => name;\n";
    let spans = highlight(LanguageId::JavaScript, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 0, "title"),
        Some("variable.parameter")
    );
}

#[test]
fn highlights_typescript_parameter_patterns() {
    let text = "function greet({ name, city = \"x\", ...extra }: User, [first, ...rest]: string[], ...others: number[]) {}\n";
    let spans = highlight(LanguageId::TypeScript, text, 0, 1);
    for parameter in ["name", "city", "extra", "first", "rest", "others"] {
        assert_eq!(
            name_of(&spans, text, 0, parameter),
            Some("variable.parameter"),
            "{parameter}"
        );
    }
}

#[test]
fn highlights_python_splat_parameters_and_class_bases() {
    let text = "class Item(Base, metaclass=Meta):\n    def load(self, *args: int, **kwargs: str):\n        pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "Base"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "Meta"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "args"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 1, "kwargs"),
        Some("variable.parameter")
    );
}

#[test]
fn highlights_java_modules_constants_and_members() {
    let text = "package demo.app;\nimport java.util.List;\nclass Box {\n    private static final int LIMIT = 10;\n    private String value;\n    void show() { this.value = other.label; }\n}\n";
    let spans = highlight(LanguageId::Java, text, 0, 7);
    for module in ["demo", "app"] {
        assert_eq!(name_of(&spans, text, 0, module), Some("module"), "{module}");
    }
    for module in ["java", "util", "List"] {
        assert_eq!(name_of(&spans, text, 1, module), Some("module"), "{module}");
    }
    assert_eq!(name_of(&spans, text, 3, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 4, "value"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "label"), Some("property"));
}

#[test]
fn highlights_deep_scoped_modules() {
    let java = "import org.springframework.boot.autoconfigure.SpringBootApplication;\n";
    let spans = highlight(LanguageId::Java, java, 0, 1);
    for module in [
        "org",
        "springframework",
        "boot",
        "autoconfigure",
        "SpringBootApplication",
    ] {
        assert_eq!(name_of(&spans, java, 0, module), Some("module"), "{module}");
    }

    let csharp = "using Microsoft.AspNetCore.Mvc.ModelBinding;\n";
    let spans = highlight(LanguageId::CSharp, csharp, 0, 1);
    for module in ["Microsoft", "AspNetCore", "Mvc", "ModelBinding"] {
        assert_eq!(
            name_of(&spans, csharp, 0, module),
            Some("module"),
            "{module}"
        );
    }
}

#[test]
fn highlights_csharp_modules_members_and_enum_members() {
    let text = "using System.Collections.Generic;\nnamespace Demo.App;\nenum Color { Red }\nclass Box {\n    private string value;\n    void Show() { this.value = other.label; System.Console.WriteLine(label); }\n}\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 7);
    for module in ["System", "Collections", "Generic"] {
        assert_eq!(name_of(&spans, text, 0, module), Some("module"), "{module}");
    }
    for module in ["Demo", "App"] {
        assert_eq!(name_of(&spans, text, 1, module), Some("module"), "{module}");
    }
    assert_eq!(name_of(&spans, text, 2, "Red"), Some("constant"));
    assert_eq!(name_of(&spans, text, 4, "value"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "label"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "WriteLine"), Some("function"));
}

#[test]
fn highlights_go_function_method_and_interface_names() {
    let text = "package main\nfunc build() {}\ntype Box struct {}\nfunc (b Box) Show() {}\ntype Shape interface { Draw() }\n";
    let spans = highlight(LanguageId::Go, text, 0, 5);
    assert_eq!(name_of(&spans, text, 1, "build"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "Show"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 4, "Draw"), Some("function.method"));
}

#[test]
fn highlights_rust_uppercase_constants() {
    let text = "const LIMIT: usize = 10;\nenum State { Ready }\n";
    let spans = highlight(LanguageId::Rust, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 1, "State"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "Ready"), Some("constant"));
}

#[test]
fn highlights_python_plain_parameters() {
    let text = "def greet(name):\n    return name\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "return"), Some("keyword.return"));
}

#[test]
fn highlights_python_literal_types() {
    let text = "def render(value: Literal[None], label: typing.Literal[\"a\"]):\n    pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "Literal"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "None"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 0, "typing"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "\"a\""), Some("string"));
}

#[test]
fn highlights_python_union_types() {
    let text = "def render(value: int | None, other: Foo | list[int]):\n    pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "None"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 0, "Foo"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "list"), Some("type.builtin"));
}

#[test]
fn highlights_python_nested_typing_types() {
    let text = "def load(value: typing.Dict[str, typing.List[int]]):\n    pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "typing"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "Dict"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "List"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "int"), Some("type.builtin"));
}

#[test]
fn highlights_python_annotated_metadata() {
    let text = "def annotate(value: typing.Annotated[int, \"meta\"]):\n    pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "typing"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "\"meta\""), Some("string"));
}

#[test]
fn highlights_python_callable_types() {
    let text = "def call(value: typing.Callable[[int], str]):\n    pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "typing"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "Callable"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "str"), Some("type.builtin"));
    let brackets = text.find("[[").expect("callable brackets");
    assert_eq!(name_at(&spans, 0, brackets), Some("punctuation.bracket"));
    assert_eq!(
        name_at(&spans, 0, brackets + 1),
        Some("punctuation.bracket")
    );
}

#[test]
fn highlights_ruby_locals_calls_and_constants() {
    let text = "require \"json\"\nclass HTTPServer\n  LIMIT = 10\nend\ndef show(name)\n  puts name\n  object.show(name)\nend\n";
    let spans = highlight(LanguageId::Ruby, text, 0, 8);
    assert_eq!(name_of(&spans, text, 0, "require"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "HTTPServer"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 4, "show"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "name"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 5, "puts"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 5, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 6, "object"), Some("variable"));
    assert_eq!(name_of(&spans, text, 6, "show"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 6, "name"), Some("variable"));
}

#[test]
fn highlights_php_variables_and_properties() {
    let text = "<?php\nfunction show($count) {\n    $value = $count;\n    $this->label = $value;\n    return $value;\n}\n";
    let spans = highlight(LanguageId::Php, text, 0, 6);
    assert_eq!(
        name_of(&spans, text, 1, "count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 3, "label"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "value"), Some("variable"));
}

#[test]
fn highlights_toml_table_and_bare_keys() {
    let text = "[package]\nname = \"kawi\"\n";
    let spans = highlight(LanguageId::Toml, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "package"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "name"), Some("property"));
}

#[test]
fn highlights_escape_sequences() {
    let cases = [
        (LanguageId::Rust, "let value = \"a\\nb\";\n", 0, "\\n"),
        (LanguageId::Python, "value = \"a\\nb\"\n", 0, "\\n"),
        (LanguageId::Json, "{\"value\": \"a\\nb\"}\n", 0, "\\n"),
        (
            LanguageId::Go,
            "package main\nvar value = \"a\\nb\"\n",
            1,
            "\\n",
        ),
        (LanguageId::Ruby, "value = \"a\\nb\"\n", 0, "\\n"),
        (LanguageId::C, "const char *value = \"a\\nb\";\n", 0, "\\n"),
        (
            LanguageId::Cpp,
            "const char *value = \"a\\nb\";\n",
            0,
            "\\n",
        ),
        (LanguageId::Toml, "value = \"a\\nb\"\n", 0, "\\n"),
        (LanguageId::Yaml, "value: \"a\\nb\"\n", 0, "\\n"),
        (LanguageId::Css, "a { content: \"\\a\"; }\n", 0, "\\a"),
    ];
    for (language, text, row, needle) in cases {
        let spans = highlight(language, text, 0, text.lines().count());
        assert_eq!(
            name_of(&spans, text, row, needle),
            Some("string.escape"),
            "{language:?}"
        );
    }
}
