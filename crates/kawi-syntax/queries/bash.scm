(variable_name) @variable
((command_name (word) @function.builtin)
 (#any-of? @function.builtin
  "alias" "bg" "bind" "break" "builtin" "cd" "command" "continue" "declare"
  "dirs" "disown" "echo" "enable" "eval" "exec" "exit" "export" "fc" "fg"
  "getopts" "hash" "help" "history" "jobs" "kill" "let" "local" "logout"
  "popd" "printf" "pushd" "pwd" "read" "readonly" "set" "shift" "shopt"
  "suspend" "test" "times" "trap" "type" "typeset" "ulimit" "umask" "unalias"
  "unset" "wait"))
((command_name (word) @keyword.return)
 (#eq? @keyword.return "return"))
((command_name (word) @keyword.import)
 (#eq? @keyword.import "source"))
"function" @keyword.function
"local" @keyword.storage
"declare" @keyword.storage
"readonly" @keyword.storage
"typeset" @keyword.storage
