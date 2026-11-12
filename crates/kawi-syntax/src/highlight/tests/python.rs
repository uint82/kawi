use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_python_naming_and_literals() {
    let text = "class HTTPServer:\n    LIMIT = 1\n\n__version__ = \"1.0\"\n\ndef helper(value=1.5):\n    flag = True\n    missing = None\n    text = \"hi\\n\"\n    _ = value\n    return f\"{value}\"\n";
    let spans = highlight(LanguageId::Python, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "HTTPServer"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 1, "1"), Some("number"));
    assert_eq!(
        name_of(&spans, text, 3, "__version__"),
        Some("constant.builtin")
    );
    assert_eq!(name_of(&spans, text, 5, "helper"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 5, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 5, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 6, "True"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 7, "None"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 8, "hi"), Some("string"));
    assert_eq!(name_of(&spans, text, 8, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 10, "{"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 10, "}"), Some("punctuation.special"));
}

#[test]
fn highlights_python_parameters_calls_and_members() {
    let text = "class Box:\n    def load(self, name: str = \"x\", *args: int, **kwargs: str) -> str:\n        self.payload = helper(name)\n        obj.method(self.payload)\n        Item()\n        print(isinstance(name, Item))\n        return self.payload\n";
    let spans = highlight(LanguageId::Python, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "load"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 1, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 1, "name"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "args"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "int"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 1, "kwargs"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "payload"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 2, "helper"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 2, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "obj"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 3, "method"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 3, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 4, "Item"), Some("constructor"));
    assert_eq!(name_of(&spans, text, 5, "print"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 5, "isinstance"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 5, "Item"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "return"), Some("keyword.return"));
}

#[test]
fn highlights_python_decorators_and_type_definitions() {
    let text = "from typing import TypeAlias, TypeVar, NewType\n\nUserId: TypeAlias = int\nPoint = TypeVar(\"Point\")\nName = NewType(\"Name\", str)\n\nclass Service(Base):\n    limit = 1\n\n    @property\n    def title(self):\n        return self.payload\n\n    @cache\n    def load(self, item):\n        self.payload = item\n\n    def run(self):\n        pass\n\n    def __init__(self):\n        pass\n";
    let spans = highlight(LanguageId::Python, text, 0, 21);
    assert_eq!(name_of(&spans, text, 0, "typing"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "UserId"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 2, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "Point"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 4, "Name"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 4, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 6, "Service"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "Base"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "limit"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 9, "@"), Some("attribute"));
    assert_eq!(
        name_of(&spans, text, 9, "property"),
        Some("attribute.builtin")
    );
    assert_eq!(name_of(&spans, text, 10, "title"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 11, "payload"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 13, "cache"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 14, "load"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 14, "item"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 15, "payload"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 17, "run"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 20, "__init__"), Some("constructor"));
}

#[test]
fn highlights_python_keywords_imports_and_operators() {
    let text = "from __future__ import annotations\nimport numpy as np\nfrom pathlib import Path\n\nasync def fetch(items):\n    try:\n        for item in items:\n            if item is not None:\n                await item\n            elif item == 0:\n                continue\n            else:\n                break\n    except ValueError as error:\n        raise error from None\n    finally:\n        pass\n    return [item async for item in items]\n\ndef choose(value):\n    match value:\n        case 1 | 2:\n            yield from value\n        case _:\n            return value\n";
    let spans = highlight(LanguageId::Python, text, 0, 24);
    assert_eq!(name_of(&spans, text, 0, "from"), Some("keyword.import"));
    assert_eq!(
        name_of(&spans, text, 0, "__future__"),
        Some("module.builtin")
    );
    assert_eq!(name_of(&spans, text, 1, "numpy"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "np"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "pathlib"), Some("module"));
    assert_eq!(name_of(&spans, text, 4, "async"), Some("keyword.coroutine"));
    assert_eq!(name_of(&spans, text, 4, "fetch"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 4, "items"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 5, "try"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 6, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 6, "in"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 7, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 7, "is"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 7, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 8, "await"), Some("keyword.coroutine"));
    assert_eq!(
        name_of(&spans, text, 9, "elif"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 10, "continue"),
        Some("keyword.repeat")
    );
    assert_eq!(
        name_of(&spans, text, 11, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 12, "break"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 13, "except"),
        Some("keyword.exception")
    );
    assert_eq!(
        name_of(&spans, text, 13, "ValueError"),
        Some("type.builtin")
    );
    assert_eq!(name_of(&spans, text, 13, "as"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 14, "raise"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 14, "from"), Some("keyword.exception"));
    assert_eq!(
        name_of(&spans, text, 15, "finally"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 16, "pass"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 17, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 17, "async"),
        Some("keyword.coroutine")
    );
    assert_eq!(name_of(&spans, text, 17, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 17, "in"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 19, "choose"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 19, "value"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 20, "match"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 21, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 21, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 21, "|"), Some("operator"));
    assert_eq!(name_of(&spans, text, 22, "yield"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 22, "from"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 23, "_"), Some("character.special"));
    assert_eq!(name_of(&spans, text, 24, "return"), Some("keyword.return"));
}

#[test]
fn highlights_python_regex_docstrings_and_directives() {
    let text = "#!/usr/bin/env python3\n\"\"\"Module docs.\"\"\"\n\nimport re\n\ndef compile_pattern(value):\n    return re.compile(r\"^[a-z]+$\")\n\ndef substitute(value):\n    # keep the pattern readable\n    return re.sub(\"a\" r\"b\", \"\", value)\n";
    let spans = highlight(LanguageId::Python, text, 0, 10);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword.directive"));
    assert_eq!(name_at(&spans, 1, 0), Some("string.documentation"));
    assert_eq!(name_of(&spans, text, 3, "re"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 5, "compile_pattern"),
        Some("function")
    );
    assert_eq!(
        name_of(&spans, text, 5, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 6, "re.compile"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 6, "compile"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 6, "^[a-z]+$"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 8, "substitute"), Some("function"));
    assert_eq!(name_at(&spans, 9, 4), Some("comment"));
    assert_eq!(name_of(&spans, text, 10, "re.sub"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 10, "sub"),
        Some("function.method.call")
    );
    let column = text
        .lines()
        .nth(10)
        .expect("substitute body")
        .find("\"b\"")
        .expect("second pattern")
        + 1;
    assert_eq!(name_at(&spans, 10, column), Some("string.regexp"));
}
