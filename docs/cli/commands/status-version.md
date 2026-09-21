# `dx status` And `dx version`

Implementation status: implemented.
User reference for the consolidated diagnostics surface and single-version
pin/launcher with rollback. Internals live in
[Diagnostics And Versioning](../../contributing/diagnostics-versioning.md);
this page owns invocation, output shapes, and exit codes.

## `dx status`

`dx status [--output text|json]` reports toolchain resolution, platform/host
coverage, missing tools, and pin staleness. It takes no scope and no
`--check`, `--report`, `--fail-on`, `--pin`, `--rollback`, or `--configured`;
those fail pre-exec (exit 2).

Text mode prints one line per check:

```text
<name>: <ok|warn|error> (<detail>) hint: <hint>
```

JSON mode streams the NDJSON envelope on stdout: `command_started`, then one
`status` event per check (`name`, `status`, `detail`, `hint`), then
`command_finished` (see the [output protocol](../output-protocol.md#status)).
The check set is `toolchain`, `platform`, `tools`, `pin`; see the
contributing page for vocabulary and platform-evidence limits.

`dx status --dry-run` plans without reading the pin or computing checks:
text prints `would report status`, JSON emits only `command_started`
(`dry_run=true`) plus `command_finished`. Dry-run plans are summaries,
suppressed under `--quiet`; live status output always prints.

Exit `0` when every check passes; exit `1` (operational) when any check is
`error` — today that is pin mismatch, with a stderr hint pointing at
`dx version --pin`. Usage errors exit `2`.

## Failure explainer

There is no `dx doctor` per [ADR 0006](../../decisions/0006-cli-command-surface.md).
`dx status` plus sanitized JSON `error` events are the failure explainer:
a nonzero Bazel subprocess emits `bazel_failed` with `phase: execute` (managed:
`collect`) before `command_finished`, naming the failed command and phase plus
the stderr pointer without argv, option values, env values, or raw tool output.
Correlate the failed scope from the preceding `operation` event, check
`dx status` for toolchain/platform/pin, read the detailed Bazel diagnostic on
stderr, and rerun the Bazel verb directly for `aquery`/sandbox/cache
introspection outside the `dx` API (raw BEP is never part of the API).

## `dx version`

`dx version` prints the single version (`dx`, `rules_dx`, pin, all `0.0.0`
today). `dx version --check` validates the checked-in `.dx/version` pin
against the module version without mutating: prints `version ok` (exit `0`)
or `version drift` on stderr (exit `1`). The vendored preset fragment
stamps the same single version in its header (no new pin file); freshness
is gated by [`dx update --check`](audit-update-bazel.md#dx-update) plus
this pin check and the skew gate below. `dx version --pin <ver>` bumps from
verified release artifacts only; `dx version --rollback` re-pins the
recorded previous release. `--pin` and `--rollback` are mutually exclusive,
neither combines with `--check` (exit `2`), and both accept `--dry-run`
(`would pin …` without writing). Bare `dx version --dry-run` prints
`would report version` and `dx version --check --dry-run` prints
`would check version pin` without reading the pin; dry-run plans are
summaries, suppressed under `--quiet`. A pin that does not equal the module
version, or a rollback with nothing to restore, fails operationally
(exit `1`).

## Startup skew gate

Every workspace command checks the `.dx/version` pin at startup, before
any Bazel work starts (one file read, no subprocesses). On skew the
diagnostic names the three versions (binary, pin, module) and the repair:

```text
dx: version skew: binary 0.0.0 pin 9.9.9 module 0.0.0; fix with `dx version --pin 0.0.0` or `dx version --rollback`
```

Fail vs warn follows command class (decided under closed issue #457,
implemented in `cli/cli/src/skew.rs`):

- Proceed silently: `version`, `status` (the diagnose/repair path),
  `completion` (no version semantics).
- Warn on stderr and proceed: `check`, `audit`, `owners`, `deps`, `why`
  (read-only), plus any `--dry-run` preview (never mutates).
- Refuse (exit `1`, with a `version_skew` error event in JSON mode):
  every other command.

A missing or empty pin is a never-pinned tree, not skew, so fresh
checkouts proceed.
