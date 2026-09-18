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

JSON mode prints `{"checks":[{"name","status","detail","hint}]}` on stdout.
The check set is `toolchain`, `platform`, `tools`, `pin`; see the
contributing page for vocabulary and platform-evidence limits.

Exit `0` when every check passes; exit `1` (operational) when any check is
`error` — today that is pin mismatch, with a stderr hint pointing at
`dx version --pin`. Usage errors exit `2`.

## `dx version`

`dx version` prints the single version (`dx`, `rules_dx`, pin, all `0.0.0`
today). `dx version --check` validates the checked-in `.dx/version` pin
against the module version without mutating: prints `version ok` (exit `0`)
or `version drift` on stderr (exit `1`). `dx version --pin <ver>` bumps from
verified release artifacts only; `dx version --rollback` re-pins the
recorded previous release. `--pin` and `--rollback` are mutually exclusive,
neither combines with `--check` (exit `2`), and both accept `--dry-run`
(`would pin …` without writing). A pin that does not equal the module
version, or a rollback with nothing to restore, fails operationally
(exit `1`).

## Startup skew gate

Every workspace command checks the `.dx/version` pin at startup, before
any Bazel work starts (one file read, no subprocesses). On skew the
diagnostic names the three versions (binary, pin, module) and the repair:

```text
dx: version skew: binary 0.0.0 pin 9.9.9 module 0.0.0; fix with `dx version --pin 0.0.0` or `dx version --rollback`
```

Fail vs warn follows command class (decided in
open work):

- Proceed silently: `version`, `status` (the diagnose/repair path),
  `completion` (no version semantics).
- Warn on stderr and proceed: `check`, `audit`, `owners`, `deps`, `why`
  (read-only), plus any `--dry-run` preview (never mutates).
- Refuse (exit `1`, with a `version_skew` error event in JSON mode):
  every other command.

A missing or empty pin is a never-pinned tree, not skew, so fresh
checkouts proceed.

There is no `dx doctor` per [ADR 0006](../../decisions/0006-cli-command-surface.md).
