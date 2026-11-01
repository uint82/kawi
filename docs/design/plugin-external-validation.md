# Plugin external validation pilot

Status: ready to recruit. No independent author is enrolled yet.

## Purpose

Prove that an independent author can build, publish, and use a real WebAssembly plugin through the documented path. This pilot supplies the external validation gate in [plugin-deprecation](plugin-deprecation.md).

The pilot measures the authoring path, not the runtime feature list. Runtime and conformance work are already complete.

## Prerequisites

- A tagged Kawi release with the WASM runtime enabled by default.
- `kawi-plugin-sdk` 0.2.0 published.
- An independent author selected.
- A pilot tracking issue opened.
- The feedback form available.

## Scope

- One independent author.
- One real plugin with a workflow outside the conformance fixtures.
- One Kawi release cycle.
- Friction tracking for authoring, SDK helpers, install, grants, inspect, logs, and migration.

The pilot does not add capabilities, change the WIT contract, or remove the process runtime.

## Independence

- The author has not contributed to the Kawi host, plugin runtime, or SDK.
- The plugin is not a test fixture, example copy, or scaffold output with a new name.
- The plugin is used in a real workflow.
- The author can publish the plugin source or a release artifact.

## Release cycle

- Start: the first tagged Kawi release with the WASM runtime enabled by default and `kawi-plugin-sdk` 0.2.0.
- Use: the author installs the plugin through the documented path and uses it for at least 30 days.
- Upgrade: the author upgrades to the next tagged Kawi release and continues using the plugin.
- End: exit review after the upgrade.
- If no next release exists within 60 days, record release cadence as a process friction item and extend the pilot.

## Pilot status

| Field | Value |
| --- | --- |
| Status | Recruiting |
| Author | Unassigned |
| Reviewer | Unassigned |
| Plugin | Unassigned |
| Kawi version | TBD |
| SDK version | 0.2.0 |
| Start | TBD |
| Exit review | TBD |

## Tracking issue

Open one issue titled `[plugin pilot] <plugin>` and keep these fields current:

- Author and plugin repository.
- Kawi version and SDK version.
- Start date and planned exit review date.
- Weekly review dates.
- Links to friction issues and fix pull requests.
- Exit decision and evidence.

## Roles

- Maintainer: recruits the author, provides support, triages friction, fixes blockers, and records the decision.
- Pilot author: builds and uses the plugin, files friction reports, and completes the exit review.
- Reviewer: a second maintainer or contributor who confirms the exit evidence.

## Friction tracking

File one issue per friction item with the [plugin pilot feedback form](https://github.com/uint82/kawi/issues/new?template=plugin-pilot-feedback.yml).

Areas:

- Authoring guide.
- SDK helpers.
- Install.
- Grants.
- Inspect.
- Logs.
- Migration.

Severity:

- Blocker: the author cannot build, publish, install, grant, inspect, log, or use the plugin, and no workaround exists.
- Major: a workaround exists, but it needs maintainer help or undocumented steps.
- Minor: a documented workaround exists, or the issue is ergonomic.
- Observation: a suggestion that does not block the pilot.

Maintainer response:

- Triage each issue within two business days.
- Record the workaround and the affected version.
- Link the fix pull request to the issue.
- Update the guide, SDK, or command when the issue is resolved.

Security issues do not enter the public friction log. Report them through the repository security advisory flow.

Weekly review:

- Review open issues, blockers, workarounds, and documentation changes.
- Confirm that private help has not replaced a documented path.
- Record the review in the pilot issue.

## Exit criteria

- The plugin is built from the documented path without private help.
- The plugin is published and installed through the documented path.
- Grants, inspect, logs, and migration work as documented.
- The plugin is used for one release cycle.
- No blocker is open.
- Every major friction item has a merged fix or an accepted, documented workaround.
- The author would recommend the path to another author.
- The reviewer confirms the evidence.

## Failure and repeat

If a blocker remains or the author cannot complete the cycle, record the cause, fix it, and run another pilot. External validation stays pending.

## After the pilot

Mark external validation complete in [plugin-deprecation](plugin-deprecation.md). Start process runtime deprecation warnings for at least two minor releases. Do not remove the process runtime as part of this pilot.

## Non-goals

- New capabilities or WIT changes.
- Process runtime removal.
- A general beta program.
- Non-Rust SDK support.

## Evidence

- Plugin repository:
- Pilot issue:
- Friction issues:
- Fix pull requests:
- Exit review:
