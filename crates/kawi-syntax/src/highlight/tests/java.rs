use super::*;
use support::{highlight, name_of};

#[test]
fn highlights_java_packages_imports_and_modules() {
    let text = r#"package demo.app;
import java.util.List;
import static java.util.Collections.emptyList;
import org.springframework.boot.autoconfigure.SpringBootApplication;
class Box { }
"#;
    let spans = highlight(LanguageId::Java, text, 0, 5);
    for module in ["demo", "app"] {
        assert_eq!(name_of(&spans, text, 0, module), Some("module"), "{module}");
    }
    for module in ["java", "util", "List"] {
        assert_eq!(name_of(&spans, text, 1, module), Some("module"), "{module}");
    }
    assert_eq!(name_of(&spans, text, 2, "static"), Some("keyword.modifier"));
    for module in ["java", "util", "Collections", "emptyList"] {
        assert_eq!(name_of(&spans, text, 2, module), Some("module"), "{module}");
    }
    for module in [
        "org",
        "springframework",
        "boot",
        "autoconfigure",
        "SpringBootApplication",
    ] {
        assert_eq!(name_of(&spans, text, 3, module), Some("module"), "{module}");
    }

    let module_info = r#"module com.example.app {
    requires transitive java.sql;
    exports com.example.api;
    opens com.example.model;
}
"#;
    let spans = highlight(LanguageId::Java, module_info, 0, 4);
    assert_eq!(
        name_of(&spans, module_info, 0, "module"),
        Some("keyword.import")
    );
    for module in ["com", "example", "app"] {
        assert_eq!(
            name_of(&spans, module_info, 0, module),
            Some("module"),
            "{module}"
        );
    }
    assert_eq!(
        name_of(&spans, module_info, 1, "requires"),
        Some("keyword.import")
    );
    assert_eq!(
        name_of(&spans, module_info, 1, "transitive"),
        Some("keyword.modifier")
    );
    for module in ["java", "sql"] {
        assert_eq!(
            name_of(&spans, module_info, 1, module),
            Some("module"),
            "{module}"
        );
    }
    assert_eq!(
        name_of(&spans, module_info, 2, "exports"),
        Some("keyword.import")
    );
    for module in ["com", "example", "api"] {
        assert_eq!(
            name_of(&spans, module_info, 2, module),
            Some("module"),
            "{module}"
        );
    }
    assert_eq!(
        name_of(&spans, module_info, 3, "opens"),
        Some("keyword.import")
    );
    for module in ["com", "example", "model"] {
        assert_eq!(
            name_of(&spans, module_info, 3, module),
            Some("module"),
            "{module}"
        );
    }
}

#[test]
fn highlights_java_types_members_and_constructors() {
    let text = r#"public final class Box<T extends Number> implements Api {
    private static final int LIMIT = 10;
    private String value;
    Box(String input) { this.value = input; }
    String value() { return value; }
    class Inner { }
    interface Nested { }
    enum Kind { ONE, TWO }
    record Point(int x, int y) { }
    @interface Marker { }
}
"#;
    let spans = highlight(LanguageId::Java, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "public"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 0, "final"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 0, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 0, "extends"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "Number"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "implements"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "Api"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 1, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "String"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "String"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 3, "input"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 4, "value"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 5, "Inner"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "Nested"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "Kind"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "ONE"), Some("constant"));
    assert_eq!(name_of(&spans, text, 7, "TWO"), Some("constant"));
    assert_eq!(name_of(&spans, text, 8, "record"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 8, "Point"), Some("type"));
    assert_eq!(name_of(&spans, text, 8, "x"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 8, "y"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 9, "@interface"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 9, "Marker"), Some("type"));
}

#[test]
fn highlights_java_calls_references_and_member_access() {
    let text = r#"class Calls {
    void run() {
        this.value = other.label;
        Box.create(1);
        factory.build().start();
        var box = new Box(1);
        Runnable task = this::run;
        Function<String, Integer> parse = Integer::parseInt;
        System.out.println(value);
        super.toString();
        items[0].name = list.get(0).value;
        Object reference = other::label;
    }
}
"#;
    let spans = highlight(LanguageId::Java, text, 0, 12);
    assert_eq!(name_of(&spans, text, 2, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 2, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 2, "other"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "label"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "Box"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 3, "create"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 4, "factory"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 4, "build"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 4, "start"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 5, "var"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "box"), Some("variable"));
    assert_eq!(name_of(&spans, text, 5, "new"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 5, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "Runnable"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "task"), Some("variable"));
    assert_eq!(name_of(&spans, text, 6, "this"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 6, "run"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 7, "Function"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "Integer"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "parse"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 7, "parseInt"),
        Some("function.method")
    );
    assert_eq!(name_of(&spans, text, 8, "System"), Some("type"));
    assert_eq!(name_of(&spans, text, 8, "out"), Some("variable.member"));
    assert_eq!(
        name_of(&spans, text, 8, "println"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 9, "super"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 9, "toString"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 10, "items"), Some("variable"));
    assert_eq!(name_of(&spans, text, 10, "name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 10, "list"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 10, "get"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 10, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 11, "Object"), Some("type"));
    assert_eq!(name_of(&spans, text, 11, "reference"), Some("variable"));
    assert_eq!(name_of(&spans, text, 11, "other"), Some("variable"));
    assert_eq!(name_of(&spans, text, 11, "label"), Some("function.method"));
}
