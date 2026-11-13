use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_lua_comments_strings_and_numbers() {
    let text = "\
#!/usr/bin/env lua
-- plain comment
--- docs for the next function
-- @param value input
--[[ block
comment ]]
local text = \"line\\n\"
local count = 0xFF
local ratio = 1.5e2
";
    let spans = highlight(LanguageId::Lua, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "#!"), Some("keyword.directive"));
    assert_eq!(name_of(&spans, text, 1, "--"), Some("comment"));
    assert_eq!(
        name_of(&spans, text, 2, "---"),
        Some("comment.documentation")
    );
    assert_eq!(
        name_of(&spans, text, 3, "-- @"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 4, "--[["), Some("comment"));
    assert_eq!(name_of(&spans, text, 6, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 7, "0xFF"), Some("number"));
    assert_eq!(name_of(&spans, text, 8, "1.5e2"), Some("number"));
}

#[test]
fn highlights_lua_regex_arguments() {
    let text = "\
local pattern = string.match(\"abc123\", \"%d+\")
local found = (\"abc\"):find(\"%a+\")
local tail = string.match(\"abc\", \"%a\", 2)
local replaced = string.gsub(\"a1\", \"%d\", \"x\")
";
    let spans = highlight(LanguageId::Lua, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "\"abc123\""), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "%d+"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 1, "%a+"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 2, "%a"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 3, "%d"), Some("string.regexp"));
}

#[test]
fn highlights_lua_bracket_indexing_and_long_brackets() {
    let text = "\
local value = items[1]
local text = [=[long]=]
--[==[ block ]==]
";
    let spans = highlight(LanguageId::Lua, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "["), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, "]"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 1, "[=["), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "--[==["), Some("comment"));
}

#[test]
fn highlights_lua_method_colon_as_delimiter() {
    let text = "function M:run() end\nM:run()\n";
    let spans = highlight(LanguageId::Lua, text, 0, 2);
    let colon = text.lines().next().and_then(|line| line.find(':')).unwrap();
    assert_eq!(name_at(&spans, 0, colon), Some("punctuation.delimiter"));
    assert_eq!(
        name_of(&spans, text, 1, "run"),
        Some("function.method.call")
    );
}
