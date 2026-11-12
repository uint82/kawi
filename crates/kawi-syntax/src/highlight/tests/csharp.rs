use super::*;
use support::{highlight, name_of};

#[test]
fn highlights_csharp_namespaces_using_directives_and_aliases() {
    let text = "global using System;\nusing static System.Console;\nusing Builder = System.Text.StringBuilder;\nusing Deep = Alpha.Beta.Gamma.Delta.Epsilon.Zeta.Eta;\nnamespace Demo.App.Modules;\nclass Box { }\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "System"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "static"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 1, "Console"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "Builder"), Some("type"));
    for module in ["System", "Text", "StringBuilder"] {
        assert_eq!(name_of(&spans, text, 2, module), Some("module"), "{module}");
    }
    assert_eq!(name_of(&spans, text, 3, "Deep"), Some("type"));
    for module in ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta"] {
        assert_eq!(name_of(&spans, text, 3, module), Some("module"), "{module}");
    }
    for module in ["Demo", "App", "Modules"] {
        assert_eq!(name_of(&spans, text, 4, module), Some("module"), "{module}");
    }
    assert_eq!(name_of(&spans, text, 5, "Box"), Some("type"));
}

#[test]
fn highlights_csharp_types_members_and_constraints() {
    let text = "public sealed class Box<T> where T : class {\n    private readonly string value;\n    public List<T> Items { get; set; }\n    public event EventHandler? Changed;\n    public Box(T item) { this.value = item; }\n    public static Box<T> Create<U>(U value) where U : new() => new(value);\n    public T GetValue() => value;\n    public void Show(T item) { Console.WriteLine(value); }\n}\nrecord Point(int X, int Y);\ninterface IShape { void Draw(); }\ndelegate void Handler();\nenum Color { Red, Green }\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 13);
    assert_eq!(name_of(&spans, text, 0, "sealed"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 0, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 1, "string"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "value"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 2, "Items"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "Changed"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "EventHandler"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "Box"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 4, "item"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 5, "Create"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 5, "U"), Some("type.parameter"));
    assert_eq!(
        name_of(&spans, text, 6, "GetValue"),
        Some("function.method")
    );
    assert_eq!(name_of(&spans, text, 7, "Show"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 7, "WriteLine"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 9, "Point"), Some("type"));
    assert_eq!(name_of(&spans, text, 10, "IShape"), Some("type"));
    assert_eq!(name_of(&spans, text, 10, "Draw"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 11, "Handler"), Some("type"));
    assert_eq!(name_of(&spans, text, 12, "Red"), Some("constant"));
    assert_eq!(name_of(&spans, text, 12, "Green"), Some("constant"));
}

#[test]
fn highlights_csharp_local_functions_and_calls() {
    let text = "class Api {\n    void Local() { }\n    int Build(int count) {\n        int Twice(int number) => number * 2;\n        var handler = new Handler();\n        var created = Factory.Create<int>(count);\n        var current = handler?.Value; handler?.Invoke();\n        Twice(count);\n        return Build(0);\n    }\n}\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 11);
    assert_eq!(name_of(&spans, text, 1, "Local"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 2, "Build"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 2, "count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "Twice"), Some("function.method"));
    assert_eq!(
        name_of(&spans, text, 3, "number"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 4, "Handler"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 5, "Create"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 6, "Value"), Some("variable.member"));
    assert_eq!(
        name_of(&spans, text, 6, "Invoke"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 7, "Twice"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 8, "Build"),
        Some("function.method.call")
    );
}

#[test]
fn highlights_csharp_keywords_operators_and_preprocessor() {
    let text = "#define TRACE\n#if TRACE\nusing System;\n#endif\npublic static class Syntax {\n    public static async Task<int> Run(int value) {\n        if (value is not null and > 0) {\n            for (var i = 0; i < value; i++) { }\n            foreach (var item in Items) { }\n            try { await Work(); }\n            catch (Exception error) when (error != null) { throw; }\n            finally { return 1; }\n        }\n        switch (value) { case 1: break; default: return 0; }\n        yield return value;\n    }\n}\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 16);
    assert_eq!(
        name_of(&spans, text, 0, "define"),
        Some("keyword.directive.define")
    );
    assert_eq!(name_of(&spans, text, 0, "TRACE"), Some("constant.macro"));
    assert_eq!(name_of(&spans, text, 1, "if"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 4, "static"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 4, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 5, "async"), Some("keyword.coroutine"));
    assert_eq!(name_of(&spans, text, 5, "Task"), Some("type"));
    assert_eq!(name_of(&spans, text, 5, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "Run"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 6, "is"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 6, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 7, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 7, "var"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 8, "foreach"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 8, "in"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 9, "try"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 9, "await"), Some("keyword.coroutine"));
    assert_eq!(
        name_of(&spans, text, 9, "Work"),
        Some("function.method.call")
    );
    assert_eq!(
        name_of(&spans, text, 10, "catch"),
        Some("keyword.exception")
    );
    assert_eq!(
        name_of(&spans, text, 10, "throw"),
        Some("keyword.exception")
    );
    assert_eq!(
        name_of(&spans, text, 11, "finally"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 11, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 13, "switch"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 13, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 13, "default"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 13, "break"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 14, "yield"), Some("keyword.return"));
}

#[test]
fn highlights_csharp_literals_comments_and_attributes() {
    let text = "/// <summary>Docs.</summary>\n[Obsolete(\"use New\")]\n[return: NotNull]\npublic class Samples {\n    public string Text = \"plain\";\n    public string Name = \"value\";\n    public string Verbatim = @\"C:\\temp\";\n    public string Raw = \"\"\"raw\"\"\";\n    public string Interpolated = $\"value: {Text}\";\n    public int Count = 1_000;\n    public double Ratio = 1.5;\n    public char Letter = 'x';\n    public bool Enabled = true;\n    public object? Nothing = null;\n    public string Slice = Text[1..];\n    // line comment\n    /* block comment */\n}\n";
    let spans = highlight(LanguageId::CSharp, text, 0, 17);
    assert_eq!(
        name_of(&spans, text, 0, "///"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 1, "Obsolete"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 2, "NotNull"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 4, "Text"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 4, "\"plain\""), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "Name"), Some("variable.member"));
    assert_eq!(
        name_of(&spans, text, 6, "Verbatim"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 6, "@\""), Some("string"));
    assert_eq!(name_of(&spans, text, 7, "Raw"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 7, "\"\"\""), Some("string"));
    assert_eq!(
        name_of(&spans, text, 8, "Interpolated"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 8, "$\""), Some("string"));
    assert_eq!(name_of(&spans, text, 9, "Count"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 9, "1_000"), Some("number"));
    assert_eq!(name_of(&spans, text, 10, "Ratio"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 10, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 11, "Letter"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 11, "'x'"), Some("character"));
    assert_eq!(
        name_of(&spans, text, 12, "Enabled"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 12, "true"), Some("boolean"));
    assert_eq!(
        name_of(&spans, text, 13, "Nothing"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 13, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 14, "Slice"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 15, "//"), Some("comment"));
    assert_eq!(name_of(&spans, text, 16, "/*"), Some("comment"));
}
