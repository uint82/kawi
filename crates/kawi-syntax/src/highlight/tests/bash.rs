use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_bash_conditionals_repeats_and_declarations() {
    let text = "if [[ -n \"$name\" ]]; then\n    echo ok\nelif false; then\n    :\nelse\n    echo no\nfi\nfor item in one two; do\n    echo \"$item\"\ndone\nwhile true; do\n    break\ndone\ndeclare -r LIMIT=10\nlocal name=value\nreadonly TOTAL=0\ntypeset -i count=0\nunset name\n";
    let spans = highlight(LanguageId::Bash, text, 0, 20);
    assert_eq!(name_of(&spans, text, 0, "if"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 0, "then"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 0, "-n"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "echo"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 2, "elif"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 2, "false"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 4, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 6, "fi"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 7, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 7, "in"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 7, "do"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 9, "done"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 10, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 11, "break"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 13, "declare"),
        Some("keyword.storage")
    );
    assert_eq!(name_of(&spans, text, 14, "local"), Some("keyword.storage"));
    assert_eq!(
        name_of(&spans, text, 15, "readonly"),
        Some("keyword.storage")
    );
    assert_eq!(
        name_of(&spans, text, 16, "typeset"),
        Some("keyword.storage")
    );
    assert_eq!(name_of(&spans, text, 17, "unset"), Some("keyword"));
}

#[test]
fn highlights_bash_functions_builtins_and_imports() {
    let text = "greet() {\n    echo \"hi\"\n}\nfunction other {\n    printf '%s\\n' \"$1\"\n}\nsource ./lib.sh\n. ./env.sh\nreturn 0\n";
    let spans = highlight(LanguageId::Bash, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "greet"), Some("function"));
    assert_eq!(name_of(&spans, text, 1, "echo"), Some("function.builtin"));
    assert_eq!(
        name_of(&spans, text, 3, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 3, "other"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "printf"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 6, "source"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 7, "."), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 8, "return"), Some("keyword.return"));
}

#[test]
fn highlights_bash_variables_assignments_and_expansions() {
    let text = "name=value\nTOTAL=0\necho \"$name ${name:-fallback} $1 $@ $HOME $?\"\n";
    let spans = highlight(LanguageId::Bash, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "value"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "TOTAL"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "${"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 2, ":-"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 2, "1"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "@"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "HOME"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 2, "?"), Some("constant"));
}

#[test]
fn highlights_bash_redirects_heredocs_and_shebang() {
    let text = "#!/usr/bin/env bash\ncat < input.txt > output.txt\nexec 3> output.log\ncat <<EOF\ncontent\nEOF\n";
    let spans = highlight(LanguageId::Bash, text, 0, 6);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword.directive"));
    assert_eq!(
        name_of(&spans, text, 1, "input.txt"),
        Some("string.special.path")
    );
    assert_eq!(
        name_of(&spans, text, 1, "output.txt"),
        Some("string.special.path")
    );
    assert_eq!(name_of(&spans, text, 2, "3"), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 2, "output.log"),
        Some("string.special.path")
    );
    assert_eq!(name_of(&spans, text, 3, "EOF"), Some("label"));
    assert_eq!(name_of(&spans, text, 4, "content"), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "EOF"), Some("label"));
}

#[test]
fn highlights_bash_regex_signals_printf_targets_and_cases() {
    let text = "[[ $name =~ ^a(b|c)+$ ]]\ncase \"$1\" in\n    start) echo go ;;\n    *) exit 1 ;;\nesac\ntrap 'echo bye' SIGINT\nprintf -v output '%s' value\n";
    let spans = highlight(LanguageId::Bash, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "^a(b|c)+$"), Some("string.regexp"));
    assert_eq!(
        name_of(&spans, text, 2, "start"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 2, ";;"),
        Some("punctuation.delimiter")
    );
    assert_eq!(name_of(&spans, text, 3, "exit"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 5, "SIGINT"), Some("string.special"));
    assert_eq!(name_of(&spans, text, 6, "output"), Some("variable"));
}
