use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_ruby_strings_symbols_and_interpolation() {
    let text = r#"name = "line\nbreak #{value}"
symbol = :name
dynamic = :"name-#{value}"
hash = {name: "demo", "key" => value}
regex = /ab+c/im
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 0, "line"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 0, "#{"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 0, "value"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 1, ":name"),
        Some("string.special.symbol")
    );
    assert_eq!(
        name_of(&spans, text, 2, "name-"),
        Some("string.special.symbol")
    );
    assert_eq!(name_of(&spans, text, 2, "#{"), Some("punctuation.special"));
    assert_eq!(
        name_of(&spans, text, 3, "name"),
        Some("string.special.symbol")
    );
    let colon = text
        .lines()
        .nth(3)
        .and_then(|line| line.find("name:"))
        .unwrap()
        + 4;
    assert_eq!(name_at(&spans, 3, colon), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 3, "\"demo\""), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "=>"), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 4, "ab+c"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 4, "/"), Some("punctuation.bracket"));
}

#[test]
fn highlights_ruby_numbers_booleans_and_characters() {
    let text = r#"integer = 42
float = 1.5
complex = 1i
rational = 1r
char = ?a
flag = true
nothing = nil
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 1, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 2, "1i"), Some("number"));
    assert_eq!(name_of(&spans, text, 3, "1r"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "?a"), Some("character"));
    assert_eq!(name_of(&spans, text, 5, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 6, "nil"), Some("constant.builtin"));
}

#[test]
fn highlights_ruby_keywords_and_comments() {
    let text = r#"#!/usr/bin/env ruby
# Public API.
class Demo
  def run(value)
    if value
      return value
    else
      raise "bad"
    end
  end
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 11);
    assert_eq!(name_of(&spans, text, 0, "#!"), Some("keyword.directive"));
    assert_eq!(
        name_of(&spans, text, 1, "# Public API."),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 2, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 2, "Demo"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "def"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 3, "run"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 3, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 4, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 5, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 6, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 7, "raise"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 8, "end"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 9, "end"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 10, "end"), Some("keyword"));
}

#[test]
fn highlights_ruby_control_flow() {
    let text = r#"for item in items
  next if item
end
while ready
  break
end
case value
in Integer
  value
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 0, "in"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 1, "next"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 1, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 3, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "break"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 6, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 7, "in"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 7, "Integer"), Some("type"));
    assert_eq!(name_of(&spans, text, 9, "end"), Some("keyword"));
}

#[test]
fn highlights_ruby_operators_and_punctuation() {
    let text = r#"value = 1
value += 2
sum = value + 3
range = 1..10
choice = value > 0 ? "yes" : "no"
[1].each { |item| item }
truth = ready and not done or fallback
negated = !ready
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 8);
    assert_eq!(name_of(&spans, text, 0, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, "+="), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "+"), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, ".."), Some("operator"));
    assert_eq!(
        name_of(&spans, text, 4, "?"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(
        name_of(&spans, text, 4, ":"),
        Some("keyword.conditional.ternary")
    );
    assert_eq!(name_of(&spans, text, 5, "|"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 5, "{"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 5, "."), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 5, "item"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 6, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 6, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 6, "or"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 7, "!"), Some("operator"));
}

#[test]
fn highlights_ruby_heredocs_and_collections() {
    let text = r#"text = <<~SQL
  select 1
SQL
words = %w(one two)
syms = %i(one two)
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "<<~SQL"), Some("label"));
    assert_eq!(name_of(&spans, text, 1, "select"), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "SQL"), Some("label"));
    assert_eq!(name_of(&spans, text, 3, "%w("), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 3, "one"), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "%i("), Some("punctuation.bracket"));
    assert_eq!(
        name_of(&spans, text, 4, "one"),
        Some("string.special.symbol")
    );
}

#[test]
fn highlights_ruby_exceptions_and_remaining_control_flow() {
    let text = r#"begin
  risky
rescue StandardError => error
  warn error
ensure
  cleanup
end
if ready
  until done
    retry
  end
elsif other
  redo
else
  unless blocked
    case value
    when Integer then value
    else
      nil
    end
  end
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 22);
    assert_eq!(name_of(&spans, text, 0, "begin"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 2, "rescue"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 2, "StandardError"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "error"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 4, "ensure"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 6, "end"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 8, "until"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 9, "retry"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 11, "elsif"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 12, "redo"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 13, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 14, "unless"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 15, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 16, "when"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 16, "then"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 18, "nil"), Some("constant.builtin"));
    assert_eq!(
        name_of(&spans, text, 21, "end"),
        Some("keyword.conditional")
    );
}

#[test]
fn highlights_ruby_pattern_matching() {
    let text = r#"case value
in {name: String => name}
  name
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 4);
    assert_eq!(
        name_of(&spans, text, 0, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 1, "in"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 1, "name"),
        Some("string.special.symbol")
    );
    let colon = text
        .lines()
        .nth(1)
        .and_then(|line| line.find("name:"))
        .unwrap()
        + 4;
    assert_eq!(name_at(&spans, 1, colon), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 1, "String"), Some("type"));
    let binding = text
        .lines()
        .nth(1)
        .and_then(|line| line.rfind("name"))
        .unwrap();
    assert_eq!(name_at(&spans, 1, binding), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "name"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "end"), Some("keyword"));
}
