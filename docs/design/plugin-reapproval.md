# Batched plugin re-approval

Status: superseded by the install-time trust model. Managed plugins no longer use `permissions.toml`, per-capability approval, or re-approval prompts. The update flow now prompts once only when the declared host API surface grows.

## Problem

A plugin update changes its manifest request set. Today `permissions.toml` stores only a request hash. A hash mismatch is all or nothing: Kawi withholds every non-default grant and the user re-approves one capability at a time. That causes restarts after each approval and hides which individual capabilities actually changed.

## Stored state

`permissions.toml` stores the approved request set, not only a hash:

```toml
[plugins.example.approved]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }

[plugins.example.grant]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }
```

The approved table uses the same shape as a manifest request. The file stays user owned and is never read from a plugin directory. `request_hash` remains accepted for older files. If the approved table is missing, a matching hash approves the current request set; a missing or corrupt record with no matching hash treats every request as new.

## Diff

Kawi compares the approved request set with the current manifest request set per capability:

- added: the request is new.
- widened: the new scope contains something the approved scope did not.
- narrowed: the approved scope contains something the new request does not.
- removed: the approved request is gone.
- unchanged: the scope is equal after normalization.

Ordering is irrelevant because both sets are canonical and sorted. A scope that adds one item and removes another is treated as widened because it contains a new item that was never approved.

## Grant policy

- Unchanged requests keep their grants.
- Narrowed requests keep their grants, intersected with the new request.
- Added and widened requests are withheld until the user approves them.
- A grant is never carried over a widened scope, even if part of the grant would still fit.
- Removed requests expose no access and their grants are reported as stale.
- The default tier still applies without approval.

## Prompt

Each plugin gets one prompt for its pending changes. The prompt lists every added and widened capability with its current decision. The user can toggle one item at a time and can allow all or deny all. Process plugins offer allow once, allow always, and deny per item. WebAssembly startup prompts offer allow always and deny because linking is fixed at instantiation.

Only real keyboard input answers the prompt. Plugin names, capability names, scope labels, and change labels are sanitized before display. Synthetic keys and plugin messages cannot answer or replace the prompt.

## Persistence and restart

The approved table is rewritten from the per-item decisions in one atomic file write. Allowed added and widened requests enter the approved set. Denied additions stay absent. Denied widenings keep their previous approved scope so the next load still reports a widening. Narrowed and unchanged requests update to the current request set, and removed requests leave the approved set.

Comments and unrelated plugin grants are preserved. Kawi re-reads the file before merging. A failed write falls back to allow once for process plugins and reports an error for WebAssembly plugins.

The plugin restarts once after the batch finishes, not after each item. A plugin with queued prompts for other capabilities still waits until its queue drains.

## Tests

The suite covers added, widened, narrowed, removed, reordered, mixed, and missing or corrupt approved records. It also flips one manifest between two states repeatedly and checks that the diff repeats. Prompt tests cover per-item toggles, allow all, deny all, hidden once for WebAssembly, sanitized labels, synthetic-key rejection, and one restart after a batch.
