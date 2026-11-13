use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_yaml_mapping_keys_and_scalars() {
    let text = "\
name: demo
count: 42
ratio: 1.5
enabled: true
disabled: FALSE
missing: null
tilde: ~
text: \"line\\nbreak\"
single: 'quoted'
";
    let spans = highlight(LanguageId::Yaml, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "demo"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "count"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 2, "ratio"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "1.5"), Some("number"));
    assert_eq!(name_of(&spans, text, 3, "enabled"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 4, "disabled"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "FALSE"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 5, "missing"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 6, "tilde"), Some("property"));
    assert_eq!(name_of(&spans, text, 6, "~"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 7, "text"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, "line"), Some("string"));
    assert_eq!(name_of(&spans, text, 7, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 8, "single"), Some("property"));
    assert_eq!(name_of(&spans, text, 8, "quoted"), Some("string"));
}

#[test]
fn highlights_yaml_quoted_and_typed_keys() {
    let text = "\
\"quoted key\": 1
'single key': 2
? explicit key
: 3
true: 4
123: 5
";
    let spans = highlight(LanguageId::Yaml, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "quoted key"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "single key"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "explicit key"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "true"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "123"), Some("property"));
}

#[test]
fn highlights_yaml_flow_collections() {
    let text = "\
flow: {name: demo, count: 2, nested: {enabled: true}}
list: [one, 2, false, null]
";
    let spans = highlight(LanguageId::Yaml, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "demo"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "count"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "2"), Some("number"));
    assert_eq!(name_of(&spans, text, 0, "nested"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "enabled"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 1, "one"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "2"), Some("number"));
    assert_eq!(name_of(&spans, text, 1, "false"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 1, "null"), Some("constant.builtin"));
}

#[test]
fn highlights_yaml_anchors_tags_and_directives() {
    let text = "\
%YAML 1.2
%TAG !e! tag:example.com,2000:
---
base: &anchor
  name: !!str demo
ref: *anchor
...
";
    let spans = highlight(LanguageId::Yaml, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "%YAML"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 1, "%TAG"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 2, "---"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 3, "base"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "&"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 3, "anchor"), Some("label"));
    assert_eq!(name_of(&spans, text, 4, "name"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "!!str"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "demo"), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "ref"), Some("property"));
    assert_eq!(name_of(&spans, text, 5, "*"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 5, "anchor"), Some("label"));
    assert_eq!(name_of(&spans, text, 6, "..."), Some("punctuation.special"));
}

#[test]
fn highlights_yaml_block_scalars_and_sequence_items() {
    let text = "\
items:
  - true
  - yes
  - null
literal: |
  first
  second
folded: >
  text
";
    let spans = highlight(LanguageId::Yaml, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "items"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "-"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 1, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 2, "yes"), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 4, "literal"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "|"), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "first"), Some("string"));
    assert_eq!(name_of(&spans, text, 6, "second"), Some("string"));
    assert_eq!(name_of(&spans, text, 7, "folded"), Some("property"));
    assert_eq!(name_of(&spans, text, 7, ">"), Some("string"));
    assert_eq!(name_of(&spans, text, 8, "text"), Some("string"));
}
