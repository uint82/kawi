use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_ruby_types_constants_and_builtins() {
    let text = r#"class HTTPServer < BaseServer
  LIMIT = 10
  ENV["KEY"]
  __FILE__
  self.class
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "HTTPServer"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "BaseServer"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "ENV"), Some("constant.builtin"));
    assert_eq!(
        name_of(&spans, text, 3, "__FILE__"),
        Some("constant.builtin")
    );
    assert_eq!(name_of(&spans, text, 4, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 4, "class"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 5, "end"), Some("keyword"));
}

#[test]
fn highlights_ruby_methods_calls_and_parameters() {
    let text = r#"def show(name, count: 0, *rest, **opts, &block)
  object.show(name)
  private
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "def"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 0, "show"), Some("function"));
    assert_eq!(name_of(&spans, text, 0, "name"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 0, "count"),
        Some("variable.parameter")
    );
    let colon = text
        .lines()
        .next()
        .and_then(|line| line.find("count:"))
        .unwrap()
        + 5;
    assert_eq!(name_at(&spans, 0, colon), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 0, "rest"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 0, "opts"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 0, "block"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "object"), Some("variable"));
    assert_eq!(name_of(&spans, text, 1, "show"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 1, "name"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 2, "private"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 3, "end"), Some("keyword.function"));
}

#[test]
fn highlights_ruby_imports_and_builtins() {
    let text = r#"require "json"
require_relative "helper"
load "tasks.rb"
autoload :Thing, "thing"
include Comparable
extend self
attr_reader :name
defined?(name)
raise ArgumentError, "bad"
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "require"), Some("keyword.import"));
    assert_eq!(
        name_of(&spans, text, 1, "require_relative"),
        Some("keyword.import")
    );
    assert_eq!(name_of(&spans, text, 2, "load"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 3, "autoload"), Some("keyword.import"));
    assert_eq!(
        name_of(&spans, text, 4, "include"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 4, "Comparable"), Some("type"));
    assert_eq!(name_of(&spans, text, 5, "extend"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 5, "self"), Some("variable.builtin"));
    assert_eq!(
        name_of(&spans, text, 6, "attr_reader"),
        Some("function.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 7, "defined?"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 8, "raise"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 8, "ArgumentError"), Some("type"));
}

#[test]
fn highlights_ruby_variables_and_members() {
    let text = r#"$global = @instance
@@class = local
CONFIG.fetch(:key)
JSON.parse("{}")
HTTPServer.new
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "$global"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 0, "@instance"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 1, "@@class"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 1, "local"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "CONFIG"), Some("constant"));
    assert_eq!(name_of(&spans, text, 2, "fetch"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 3, "JSON"), Some("constant"));
    assert_eq!(name_of(&spans, text, 3, "parse"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 4, "HTTPServer"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "new"), Some("function.call"));
}

#[test]
fn highlights_ruby_definitions_and_aliases() {
    let text = r#"alias new_name old_name
def name=(value)
  @name = value
end
def self.build(...)
  target(...)
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "alias"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "new_name"), Some("function"));
    assert_eq!(name_of(&spans, text, 0, "old_name"), Some("function"));
    assert_eq!(name_of(&spans, text, 1, "name"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 1, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 2, "@name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "end"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 4, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 4, "build"), Some("function"));
    assert_eq!(name_of(&spans, text, 4, "..."), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 5, "target"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 5, "..."), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 6, "end"), Some("keyword.function"));
}

#[test]
fn highlights_ruby_block_and_lambda_parameters() {
    let text = r#"[1].each do |(left, right), rest|
  left
end
->(first, second = 1, *more, **options, &callback) { first }
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "do"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "left"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 0, "right"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 0, "rest"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "left"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 3, "first"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 3, "second"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "more"), Some("variable.parameter"));
    assert_eq!(
        name_of(&spans, text, 3, "options"),
        Some("variable.parameter")
    );
    assert_eq!(
        name_of(&spans, text, 3, "callback"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "->"), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "{"), Some("punctuation.bracket"));
}

#[test]
fn highlights_ruby_modules_returns_and_super() {
    let text = r#"module Demo
  class Box
    def show(value)
      yield value
      super(value)
      return value
    end
  end
end
"#;
    let spans = highlight(LanguageId::Ruby, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "module"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "Demo"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "class"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 1, "Box"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "def"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 2, "show"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 2, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 3, "yield"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 4, "super"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 5, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 6, "end"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 7, "end"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 8, "end"), Some("keyword"));
}
