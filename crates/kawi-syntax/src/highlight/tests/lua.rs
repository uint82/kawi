use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_lua_functions_and_members() {
    let text = "\
local M = {}
local LIMIT = 10

function M.show(item)
    print(item.name)
    M.show(item)
end

function M:run()
    self.value = self:load()
end

M.handler = function()
    return M.handler
end

local handlers = { show = function() end }
return M
";
    let spans = highlight(LanguageId::Lua, text, 0, 18);
    assert_eq!(name_of(&spans, text, 1, "LIMIT"), Some("constant"));
    assert_eq!(name_of(&spans, text, 3, "show"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "item"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 4, "print"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 4, "name"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 5, "show"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 8, "run"), Some("function.method"));
    assert_eq!(name_of(&spans, text, 9, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 9, "value"), Some("variable.member"));
    assert_eq!(
        name_of(&spans, text, 9, "load"),
        Some("function.method.call")
    );
    assert_eq!(name_of(&spans, text, 12, "handler"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 12, "function"),
        Some("keyword.function")
    );
    assert_eq!(
        name_of(&spans, text, 13, "handler"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 14, "end"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 16, "show"), Some("function"));
    assert_eq!(name_of(&spans, text, 16, "{"), Some("constructor"));
}

#[test]
fn highlights_lua_builtins_modules_and_literals() {
    let text = "\
local version = _VERSION
local duplicate = table.copy({ key = value, [key] = 1 })
local flag = math.huge > 0 and not false or nil
local template = string.format
local task = coroutine.create(function() end)
local loaded = require(\"mod\")
local globals = _G
local bytes = utf8.len(\"é\")
local maxSize = 10
local MAX_SIZE = 10
";
    let spans = highlight(LanguageId::Lua, text, 0, 10);
    assert_eq!(
        name_of(&spans, text, 0, "_VERSION"),
        Some("constant.builtin")
    );
    assert_eq!(name_of(&spans, text, 1, "table"), Some("module.builtin"));
    assert_eq!(name_of(&spans, text, 1, "copy"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 1, "key"), Some("property"));
    assert_eq!(name_of(&spans, text, 1, "value"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "math"), Some("module.builtin"));
    assert_eq!(name_of(&spans, text, 2, "huge"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 2, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "false"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 2, "or"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "nil"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 3, "string"), Some("module.builtin"));
    assert_eq!(name_of(&spans, text, 3, "format"), Some("variable.member"));
    assert_eq!(
        name_of(&spans, text, 4, "coroutine"),
        Some("keyword.coroutine")
    );
    assert_eq!(name_of(&spans, text, 4, "create"), Some("function.call"));
    assert_eq!(
        name_of(&spans, text, 5, "require"),
        Some("function.builtin")
    );
    assert_eq!(name_of(&spans, text, 6, "_G"), Some("module.builtin"));
    assert_eq!(name_of(&spans, text, 7, "utf8"), Some("module.builtin"));
    assert_eq!(name_of(&spans, text, 7, "len"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 8, "maxSize"), Some("variable"));
    assert_eq!(name_of(&spans, text, 9, "MAX_SIZE"), Some("constant"));
}

#[test]
fn highlights_lua_attributes_and_varargs() {
    let text = "\
local value <const> = 1
local function pack(...)
    return ...
end
local fn = function(...)
    return ...
end
local first <const>, second <close> = 1, 2
";
    let spans = highlight(LanguageId::Lua, text, 0, 8);
    assert_eq!(name_of(&spans, text, 0, "const"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, "<"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, ">"), Some("punctuation.bracket"));
    assert_eq!(
        name_of(&spans, text, 1, "..."),
        Some("variable.parameter.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 2, "..."),
        Some("variable.parameter.builtin")
    );
    assert_eq!(
        name_of(&spans, text, 4, "function"),
        Some("keyword.function")
    );
    assert_eq!(
        name_of(&spans, text, 5, "..."),
        Some("variable.parameter.builtin")
    );
    assert_eq!(name_of(&spans, text, 7, "const"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 7, "close"), Some("attribute"));
}

#[test]
fn highlights_lua_control_flow_and_labels() {
    let text = "\
::retry::
if value then
    goto retry
elseif other then
    return
else
    break
end
do local inner = 1 end
for index = 1, 10 do end
for key, item in pairs(values) do end
while true do end
repeat until done
";
    let spans = highlight(LanguageId::Lua, text, 0, 12);
    assert_eq!(name_of(&spans, text, 0, "retry"), Some("label"));
    assert_eq!(
        name_of(&spans, text, 0, "::"),
        Some("punctuation.delimiter")
    );
    assert_eq!(name_of(&spans, text, 1, "if"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 1, "then"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 2, "goto"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "retry"), Some("label"));
    assert_eq!(
        name_of(&spans, text, 3, "elseif"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 4, "return"), Some("keyword.return"));
    assert_eq!(
        name_of(&spans, text, 5, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 6, "break"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "end"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 8, "do"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 8, "end"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 9, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 9, "do"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 9, "end"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 10, "in"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 10, "pairs"), Some("function.builtin"));
    assert_eq!(name_of(&spans, text, 11, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 11, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 12, "repeat"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 12, "until"), Some("keyword.repeat"));
}

#[test]
fn highlights_lua_operators_and_punctuation() {
    let text = "\
local result = (left + right) * 2 // 3 % 4 ^ 5 .. \"x\"
local bits = value & mask | flag ~ other << 1 >> 2
local truth = not value and #items > 0 or value ~= nil
local item = object.field
local a, b = 1, 2;
";
    let spans = highlight(LanguageId::Lua, text, 0, 5);
    for operator in ["=", "+", "*", "//", "%", "^", ".."] {
        assert_eq!(
            name_of(&spans, text, 0, operator),
            Some("operator"),
            "{operator}"
        );
    }
    for operator in ["&", "|", "~", "<<", ">>"] {
        assert_eq!(
            name_of(&spans, text, 1, operator),
            Some("operator"),
            "{operator}"
        );
    }
    assert_eq!(name_of(&spans, text, 2, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "#"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "or"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "~="), Some("operator"));
    assert_eq!(name_of(&spans, text, 0, "("), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 3, "object"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "field"), Some("variable.member"));
    let dot = text.lines().nth(3).and_then(|line| line.find('.')).unwrap();
    assert_eq!(name_at(&spans, 3, dot), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 4, ","), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 4, ";"), Some("punctuation.delimiter"));
}
