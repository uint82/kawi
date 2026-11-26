# Feedback system gaps

Status: verified audit and open TODOs. This is not a design proposal.
Verified on 2026-11-26 against `198a036a`. Line numbers are from that commit.
Reference set: LSP 3.17, Neovim, Helix, Zed, VS Code.

Read this before changing message or diagnostic behavior. Keep each TODO next to
its evidence; do not create a separate `TODO.md` that will drift.

## Verdict

Kawi's diagnostic data model, aggregation, validation, and navigation are above
average for a young terminal editor. Its LSP feedback semantics and default
display policy are not yet best practice. The principles in `feedback.md` are
sound; several are not wired through. These gaps are wiring and policy, not
architecture.

## Verified strengths

- Typed diagnostics with stable codes and severity, plus tests that enforce the
  code registry (`crates/kawi/src/app/diagnostic.rs`, `docs/design/feedback.md`).
- A unified `Problem` model across LSP, task, git, plugin, and external sources,
  with source labels, tags, related information, and stale marking
  (`crates/kawi/src/app/problems.rs`).
- A bounded 200-entry message history with clear and one-current-message
  presentation (`crates/kawi/src/app/messages.rs:10-29`).
- Deferred diagnostic updates in insert/replace mode, LSP pull diagnostics, and
  per-server progress with a statusline spinner
  (`crates/kawi/src/app/diagnostic_deferral.rs`,
  `crates/kawi-lsp/src/service/pull_diagnostics.rs`).
- Plugin diagnostics are validated and rate-limited: 1,000 per call, 4,096-byte
  messages, 256-byte codes, 20 publishes per second
  (`crates/kawi-plugin/src/protocol/constants.rs:9-11`,
  `crates/kawi/src/app/plugin/runtime/capability.rs:8-9`).
- Navigation is complete: `d d`, `d n`, `d p`, current-line popup, problems
  picker with severity and source filters, and quickfix rows showing tags,
  related counts, and stale markers (`crates/kawi/src/leader.rs`,
  `crates/kawi/src/app/quickfix.rs:198-220`).

## Gaps and TODOs

### P0: preserve LSP message severity

- [ ] Route `window/showMessage` and `window/showMessageRequest` severity into
      the typed diagnostic instead of formatting it into the message text.
  - Evidence: `crates/kawi/src/app.rs:940-942` takes `Option<String>`;
    `crates/kawi/src/app/lsp.rs:1116-1121` formats the level into `E:`/`W:`;
    `crates/kawi/src/app/messages.rs:59-61` wraps the result as information.
  - Conflict: `docs/design/feedback.md` says "Severity is never inferred from
    the message text." The LSP path does exactly that.
  - Fix: keep `MessageLevel` on the event and construct the matching
    `Diagnostic` severity; keep the server name in `source`, not in the message.
  - Test: fake LSP sends error, warning, and info messages; assert severity,
    history, and message-row rendering.

- [ ] Present `window/showMessageRequest` actions and return the selected item,
      or `null` only when the user dismisses.
  - Evidence: `crates/kawi-lsp/src/service/request.rs:43-48` always responds
    `null`; `:166-172` drops `ShowMessageRequestParams.actions`.
  - Comparison: Neovim prompts with `vim.ui.select` when actions exist
    (`runtime/lua/vim/lsp/handlers.lua`, `RSC['window/showMessageRequest']`).
  - Fix: show the actions in a picker or prompt; return the selected
    `MessageActionItem`; preserve the request severity.
  - Test: fake server sends two actions; assert the selected response and the
    dismissal response.

### P0: preserve diagnostic data for code actions

- [ ] Include `data`, tags, related information, and `codeDescription` in
      `CodeActionContext.diagnostics`.
  - Evidence: `crates/kawi-lsp/src/service/code_actions.rs:128-145` sets
    `code_description: None` and leaves the remaining fields at their defaults.
  - Impact: servers that attach `data` to diagnostics cannot round-trip it, so
    some quick fixes fail.
  - Fix: preserve the raw LSP diagnostic or add a `data` field to `Problem`;
    populate tags, related information, and `codeDescription` when rebuilding
    the context.
  - Test: fake server returns a diagnostic with `data`; request code actions;
    assert `data` is present in the context.

### P1: make diagnostic details actionable

- [ ] Open `codeDescription.href` and jump to related information locations.
  - Evidence: `crates/kawi/src/app/overlay/diagnostic.rs:74-83` prints
    `code: href` and `related: path:line: message` as plain text.
  - Comparison: LSP defines `CodeDescription.href` as a URI to open; VS Code
    models related information as locations, not text.
  - Fix: add a popup action or command to open the href and jump to each
    related location.
  - Test: popup action opens the expected path or URI.

### P1: apply severity filtering consistently

- [ ] Apply the configured severity filter to underlines and sign-column signs
      as well as virtual text.
  - Evidence: `crates/kawi/src/app/diagnostic_ui.rs:209-227` returns all
    spans; `:253` filters only virtual text;
    `crates/kawi/src/app/signcolumn.rs:85-97` applies no severity filter.
  - Comparison: Neovim supports per-handler severity filters; Helix and Zed
    expose global filters.
  - Fix: apply `diagnostic_virtual_severity` to spans and signs, or add
    separate filters if the surfaces should differ.
  - Test: a hint diagnostic is hidden from underlines and signs when the filter
    is warning.

### P1: align defaults with peers

- [ ] Decide the intended default display policy and make docs and code agree.
  - Evidence: `crates/kawi/src/app/diagnostic_ui.rs:140-150` defaults to
    virtual text on all lines at hint severity; `crates/kawi/src/options.rs:387-395`
    defaults the sign column to `never`.
  - Comparison: Neovim defaults to `virtual_text=false`, `signs=true`,
    `underline=true`; Helix defaults to inline diagnostics off and gutter
    diagnostics on; Zed defaults to inline lens off with underlines and a
    scrollbar. Kawi shows every hint inline but no persistent gutter.
  - Fix: choose and document one policy. Options include warning plus
    `cursor_line` or `off` with `signcolumn=auto`, or keep all hints but turn
    signs on.
  - Test: default options match the documented policy.

- [ ] Fix the `diagnostic_virtual_severity` docs/code mismatch.
  - Evidence: `docs/help/diagnostics.md:29` says the default is `warning`;
    `crates/kawi/src/app/diagnostic_ui.rs:143-144` defaults to `hint`.
  - Fix: align the docs and the code. If `hint` is intended, update the help
    page; if `warning` is intended, update the code and tests.
  - Test: a test covers the option default and the help text stays in sync.

### P2: complete LSP feedback methods

- [ ] Implement `window/showDocument`.
  - Evidence: no handler exists; unknown requests return `method_not_found`
    (`crates/kawi-lsp/src/service/request.rs:49`).
  - Comparison: Neovim implements `RSC['window/showDocument']`.
  - Fix: handle the request, open the URI with confirmation for external
    targets, and return success or failure.
  - Test: fake server sends `showDocument`; assert the open request and response.

- [ ] Handle `window/logMessage` in a visible log or debug surface.
  - Evidence: `crates/kawi-lsp/src/service/request.rs:89-96` emits
    `LspEvent::Log`; `crates/kawi/src/app/lsp.rs:988` ignores it.
  - Comparison: Neovim logs LSP messages at the corresponding level.
  - Fix: store log messages in `:lsp status`, `:health`, or a dedicated log
    surface; at minimum do not drop them.
  - Test: fake server sends a log message; assert it is visible.

- [ ] Evaluate `workspace/applyEdit`.
  - Evidence: no handler exists; server-initiated edits are unsupported.
  - Fix: decide whether Kawi should apply server-initiated workspace edits. If
    not, return a clear error and document the limitation.
  - Test: fake server sends `applyEdit`; assert the edit is applied or the
    error is explicit.

### P2: complete diagnostic presentation

- [ ] Show tags in the diagnostic popup.
  - Evidence: quickfix rows show tags
    (`crates/kawi/src/app/quickfix.rs:198-220`); the popup does not
    (`crates/kawi/src/app/overlay/diagnostic.rs`).
  - Fix: render deprecated and unnecessary tags alongside code and related
    information.
  - Test: deprecated and unnecessary tags are visible in the popup.

- [ ] Consider multi-line virtual text.
  - Evidence: Kawi has one-line virtual text only; Neovim also offers
    `virtual_lines`.
  - Fix: optional. Decide whether multi-line inline diagnostics are worth the
    layout cost.
  - Test: a multi-line diagnostic renders without breaking layout or scrolling.

## Deferred by contract

- Plugin message severity, code, and source are deferred until the 1.0 plugin
  review. `docs/design/feedback.md` says `UiMessageParams` keeps only `message`;
  do not add severity, code, or source before that review.
- Plugin diagnostic tags, related information, and `codeDescription` are also
  deferred until the plugin contract can change.
- `kawi:plugin@0.13.0` is frozen. Do not edit `plugin.wit` or bump the SDK
  version for these gaps.
- `ui.message` is information-only today
  (`crates/kawi/src/app/plugin/host_methods.rs:115-119`). This is expected under
  the current contract, not a bug to fix in 0.13.0.

## Peer comparison

- Neovim preserves LSP message levels, prompts for `showMessageRequest`
  actions, implements `showDocument`, and defaults to signs on with virtual text
  off.
- Helix defaults to gutter diagnostics on, inline diagnostics off, LSP messages
  displayed, and progress messages off.
- Zed defaults to inline lens off, underlines plus scrollbar, a configurable
  maximum severity, hover tooltips, and a diagnostic navigation popover.
- VS Code models related information as locations and `codeDescription` as a
  link; notifications support actions and progress.
- Kawi currently shows all hints inline, has no gutter by default, renders
  related information and links as text, and drops LSP message actions.

## Sources

- Kawi code references above.
- Neovim `runtime/lua/vim/lsp/handlers.lua` and `runtime/doc/diagnostic.txt`.
- LSP 3.17 `Diagnostic` and `CodeDescription`.
- Helix editor documentation.
- Zed diagnostics documentation.
- VS Code Diagnostic API and notification UX guidelines.

## Working agreement

- When a TODO is fixed, check it and add the commit or test that proves it.
- When a new gap is found, add it here with evidence. Do not scatter `TODO:`
  comments across the codebase; the audit keeps the context and the file
  references in one place.
