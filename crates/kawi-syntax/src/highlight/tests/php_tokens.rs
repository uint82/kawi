use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_php_keywords() {
    let text = "\
<?php
if (true) { return; } else { throw new Error(); }
for (;;) { break; }
foreach ([] as $value) { continue; }
try {} catch (Error $error) { } finally {}
echo 'x';
include 'x';
";
    let spans = highlight(LanguageId::Php, text, 0, 7);
    assert_eq!(name_of(&spans, text, 1, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 1, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 1, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 1, "throw"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 1, "new"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 2, "break"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 3, "foreach"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 3, "as"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 3, "continue"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "try"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 4, "catch"), Some("keyword.exception"));
    assert_eq!(
        name_of(&spans, text, 4, "finally"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 5, "echo"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 6, "include"), Some("keyword.import"));
}

#[test]
fn highlights_php_operators_and_punctuation() {
    let text = "\
<?php
$value = $left + $right * 2;
$value ??= $fallback;
$result = $value <=> 0;
$object?->run();
$object::run();
$array = ['key' => $value];
";
    let spans = highlight(LanguageId::Php, text, 0, 7);
    assert_eq!(name_of(&spans, text, 1, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "+"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "*"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, ";"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 2, "??="), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "<=>"), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "?->"), Some("operator"));
    assert_eq!(name_of(&spans, text, 5, "::"), Some("operator"));
    assert_eq!(name_of(&spans, text, 6, "=>"), Some("operator"));
    assert_eq!(name_of(&spans, text, 6, "["), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 6, "]"), Some("punctuation.bracket"));
}

#[test]
fn highlights_php_strings_and_literals() {
    let text = "\
<?php
$double = \"line\\nbreak\";
$single = 'quoted';
$heredoc = <<<TXT
hello
TXT;
$nowdoc = <<<'RAW'
hello
RAW;
$shell = `ls`;
$bool = true;
$nothing = null;
$int = 42;
$float = 1.5;
";
    let spans = highlight(LanguageId::Php, text, 0, 14);
    assert_eq!(name_of(&spans, text, 1, "line"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 2, "quoted"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "TXT"), Some("label"));
    assert_eq!(name_of(&spans, text, 4, "hello"), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "TXT"), Some("label"));
    assert_eq!(name_of(&spans, text, 6, "RAW"), Some("label"));
    assert_eq!(name_of(&spans, text, 7, "hello"), Some("string"));
    assert_eq!(name_of(&spans, text, 8, "RAW"), Some("label"));
    assert_eq!(name_of(&spans, text, 9, "ls"), Some("string"));
    assert_eq!(name_of(&spans, text, 10, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 11, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 12, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 13, "1.5"), Some("number.float"));
}

#[test]
fn highlights_php_comments() {
    let text = "\
<?php
// line comment
# hash comment
/** doc comment */
/* block comment */
";
    let spans = highlight(LanguageId::Php, text, 0, 5);
    assert_eq!(name_of(&spans, text, 1, "// line"), Some("comment"));
    assert_eq!(name_of(&spans, text, 2, "# hash"), Some("comment"));
    assert_eq!(
        name_of(&spans, text, 3, "/** doc"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 4, "/* block"), Some("comment"));
}

#[test]
fn highlights_php_tags_and_exit_forms() {
    let text = "\
<?php
exit;
exit(1);
?>
";
    let spans = highlight(LanguageId::Php, text, 0, 4);
    assert_eq!(
        name_of(&spans, text, 0, "<?php"),
        Some("punctuation.bracket")
    );
    assert_eq!(name_of(&spans, text, 1, "exit"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 2, "exit"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 3, "?>"), Some("punctuation.bracket"));
}

#[test]
fn highlights_php_ternary_directives_and_magic_constants() {
    let text = "\
<?php
declare(strict_types=1);
$value = $a ? $b : $c;
$value = $a ?? $b;
echo __FILE__;
";
    let spans = highlight(LanguageId::Php, text, 0, 5);
    assert_eq!(
        name_of(&spans, text, 1, "strict_types"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 2, "?"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(
        name_of(&spans, text, 2, ":"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(name_of(&spans, text, 3, "??"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 4, "__FILE__"),
        Some("constant.builtin")
    );
}

#[test]
fn highlights_php_named_arguments() {
    let text = "\
<?php
run(1, label: 'x');
";
    let spans = highlight(LanguageId::Php, text, 0, 2);
    assert_eq!(
        name_of(&spans, text, 1, "label"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "1"), Some("number"));
}
