use super::support::{highlight, name_of};
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
    assert_eq!(name_of(&spans, text, 1, "Ready"), Some("constructor"));
}
