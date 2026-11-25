# Feedback and diagnostics

Status: typed implementation. Parser failures, command errors, asynchronous service messages, and plugin messages that reach the message row create `Diagnostic` values.

## Principles

- A successful action may stay quiet. A failed action must say what failed and what the user can do next.
- Errors are data before they are text. A diagnostic has a stable code, a severity, a source, and a message. The rendered line is a view of that data.
- Codes identify behavior, not wording. Fix a typo in a message without changing its code.
- Vim compatibility is a target where the behavior matches. It is not a reason to reuse a code that means something else.
- One current message is visible. A bounded history keeps earlier messages available with `:messages`.
- The message row shows feedback. The pager shows reports and listings. They have different dismissal rules and different lifetimes.

## Code policy

### Vim-compatible codes

Use a Vim code only when the meaning matches Vim's definition. Reusing a code because it looks close is worse than using a Kawi code.

The initial command boundary uses these codes:

| Code | Meaning | Use |
| --- | --- | --- |
| `E471` | Argument required | A known command is missing a required argument. |
| `E474` | Invalid argument | A known command rejects the supplied argument. |
| `E488` | Trailing characters | A command that takes no arguments received extra text. |
| `E492` | Not an editor command | The top-level command name is unknown. |
| `E146` | Regular expressions can't be delimited by letters | A substitute pattern uses a letter as its delimiter. |
| `E939` | Positive count required | A substitute count is zero. |
| `E1510` | Value too large | A numeric argument does not fit the supported range. |

Existing codes stay in place when their meaning is correct, including `E32`, `E37`, `E89`, `E94`, `E484`, `E518`, `E939`, and `E1510`.

`E492` uses Vim's wording, "Not an editor command", not "Unknown command". `E471` replaces the old `E521: Argument required`, which used a code with a different Vim definition.

### Kawi-native codes

Use `K###` when no Vim code describes the same failure. Do not invent `E9xxx` codes. Vim reserves editor-defined ranges and future Vim versions can claim them.

Kawi codes are grouped by area:

| Range | Area |
| --- | --- |
| `K001`-`K099` | General editor state, tasks, and external tools |
| `K100`-`K199` | Files, buffers, directories, and configuration |
| `K200`-`K299` | Language services and diagnostics |
| `K300`-`K399` | Git |
| `K400`-`K499` | Plugins |

Allocated codes:

| Code | Meaning |
| --- | --- |
| `K001` | Unknown task |
| `K002` | No previous task |
| `K003` | No running task |
| `K004` | No task output |
| `K005` | A task is already running |
| `K006` | No Git editor session |
| `K007` | LSP command requires a file or server |
| `K008` | Language server service is not running |
| `K100` | No keymap file loaded |
| `K101` | Not a directory |
| `K102` | Unknown theme |
| `K103` | Unknown keymap |
| `K104` | Invalid theme file |

Rules:

- Allocate a code once and never reuse it.
- Retire a code by removing it from active paths. Do not renumber it.
- Do not use a code for a success message.
- Keep the code at the start of the first line: `CODE: message`.
- A warning may use `W###` when it has no Vim equivalent. Prefer an explicit severity over guessing from the text.

The doc is the policy. The registry in code is the source of truth.

## Diagnostic model

A diagnostic has:

- `code`: an optional stable identifier.
- `severity`: error, warning, information, or success.
- `message`: the human-readable explanation.
- `source`: built-in editor, language server, task, Git, plugin, or external file change.
- `help`: an optional help topic or command.

The implementation stores the code, severity, source, message, and help on the diagnostic. `Diagnostic::render` turns it into text only at the message row or pager. `MessageHistory` stores the typed values, so `:messages` can render them again without reparsing prefixes. Severity is never inferred from the message text.

Boundaries validate input once and produce diagnostics. Business logic returns typed results. Presentation decides color and layout. A single message should not be assembled in five places.

## Message lifecycle

Every message follows the same path:

1. A boundary creates a diagnostic with a code and severity.
2. The message sink renders it as `CODE: message` for display.
3. The sink appends it to the bounded history.
4. The sink sets it as the current message.
5. If the text needs more than one row, the pager presents it instead of the message row.
6. The next message replaces the current one. History stays.
7. `Esc` clears the current message and pager. It does not clear history.
8. `:messages` opens history in the pager. `:messages clear` clears history.

Rules:

- History holds the last 200 entries in insertion order.
- History is in memory for the session. It is not written to state files.
- Pager reports such as `:ls`, `:marks`, and `:git diff` are not messages. Do not add them to history.
- Asynchronous messages from language servers, tasks, debug, and plugins use the same sink.
- Repeated messages are kept. Deduplication hides repeated failures and makes timing harder to read.
- Multi-line diagnostics put the code on the first line. The pager stays on the base style so a long report does not become a wall of red.

Severity mapping:

| Severity | Created by | Message row |
| --- | --- | --- |
| Error | `Diagnostic::error` or a failure helper | Diagnostic error color |
| Warning | `Diagnostic::warning` | Diagnostic warning color |
| Information | `Diagnostic::information` | Base color |
| Success | `Diagnostic::success` | Base color, bold |

Color is supplementary. The code and wording must carry the meaning without it.

## Parser contract

`kawi_editor::command::parse` is pure and state-free. It returns `Result<Command, ParseFailure>`.

`ParseFailure` names the failure instead of collapsing it into one ambiguous value. The command boundary reports the failure as written and does not reclassify it.

| Failure | Meaning | Code |
| --- | --- | --- |
| `EmptyInput` | The command line is blank. | None |
| `UnknownCommand` | The top-level name is not a command. | `E492` |
| `MissingArgument` | A known command needs an argument. | `E471` |
| `InvalidArgument` | A known command rejects the supplied argument. | `E474` |
| `TrailingCharacters` | A command received text it does not accept. | `E488` |
| `RegexDelimitedByLetter` | A substitute pattern uses a letter as its delimiter. | `E146` |
| `PositiveCountRequired` | A substitute count is zero. | `E939` |
| `ValueTooLarge` | A numeric argument is outside the supported range. | `E1510` |

`parse` trims the input only to detect `EmptyInput`. It then tries file operations and named commands. Substitute names such as `s`, `su`, and `substitute` fall through to substitute parsing, including repeat forms such as `s 4`. A known command name that does not match a named form becomes `MissingArgument` or `InvalidArgument`. An unknown name becomes `UnknownCommand` after substitute parsing fails.

The command boundary keeps the raw text for suggestions:

- Unknown names produce `E492` with suggestions from built-in and plugin command names.
- Missing and invalid arguments keep the parser's code and wording.
- Empty input produces no diagnostic.
- No path may return silently when the user typed a non-empty command.

Suggestions:

- Compare the unknown name against built-in command names and registered plugin commands.
- Use case-insensitive edit distance.
- Accept candidates within two edits for names up to five characters and within three edits for longer names. Very short names get no suggestions.
- Sort by distance, then by name, and show at most three.
- Render one suggestion as `Did you mean :help?` and several as `Did you mean :help, :health?`.
- Do not suggest when the input is empty or no candidate is close.

The suggestion is part of the explanation, not a separate severity. Keep the error and the hint on one line unless the width forces a wrap.

## Command feedback

Commands that change state should report the result when the change is not visible in the buffer or status line. Saving, mapping, unmapping, task start, task cancel, and Git actions are examples.

Commands that already return a message keep doing so. The boundary only adds feedback when the parser could not produce a `Command`.

The initial cleanup replaced bare `E:` messages, stopped using `E484` for directory and keymap state, and gave unprefixed setup errors `K###` codes. Keep new failures on the same path.

## History and `:messages`

`:messages` shows every entry in the bounded history, oldest first, wrapped to the current width. It opens the pager.

`:messages clear` clears history and the current message. It produces no output.

The command is discoverable in `:commands` and command-line completion.

History is not a log file. Use `:health`, `:lsp status`, `:plugin logs`, or task output for durable diagnostics.

## Plugins

Plugin messages currently arrive as plain text without severity or code. The host wraps them as information with the plugin source. They enter history in that typed form.

The next protocol version should add optional `severity`, `code`, and `source` fields to `UiMessageParams`. Keep the current fields required for compatibility. Reject unknown severity values at the host boundary and render them as information. A plugin message must not be able to forge a built-in `E###` code without a source. Prefix plugin codes with the plugin name when they are shown to the user.

Plugin message throttling belongs to the plugin host. The message sink still records what it receives so a noisy plugin is visible in `:messages`.

## Testing

Parser tests cover:

- Every name in `COMMAND_NAMES` returns a command or a typed failure, never `UnknownCommand`.
- Empty input returns `EmptyInput`; unknown names return `UnknownCommand`.
- Known commands with missing or invalid arguments return the matching variant.
- File operations report unterminated quotes and invalid arity as typed failures.
- Substitute parsing reports `E146`, `E939`, `E1510`, and trailing characters.
- `:set` trailing characters keep `E488` and show the unexpected text, not the option name.

Suggestion tests cover:

- Exact typos at the start, middle, and end of a name.
- Case differences.
- Short names where suggestions would be noise.
- The three-suggestion cap and stable ordering.
- Built-in names and plugin command names.

Message tests cover:

- A one-line message replaces the current message.
- A multi-line message opens the pager.
- History keeps insertion order and stops at the limit.
- `:messages` opens the pager and `:messages clear` empties both history and the current message.
- A pager report does not enter history.
- `Esc` clears the message without touching history.

Rendering tests cover:

- Error and warning colors on the message row.
- Prompts, command text, search text, and search status are never tinted as diagnostics.
- Success is bold and does not rely on color.
- Existing layout and truncation behavior is unchanged.

## Rollout

1. Command boundary: return typed parser failures, report them, and suggest names. Done.
2. Message sink: typed diagnostics, bounded history, `:messages`, and message row rendering. Done.
3. Cleanup: replace bare `E:` messages, fix misused Vim codes, and route direct message writes through the sink. In progress.
4. Plugins: add typed severity and source fields to the protocol. Pending.
5. Help: add `:help errors` and resolve `:help E492` and `:help K001` through the help index. Pending.
