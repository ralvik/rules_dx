# Diagnostics And Versioning

Implementation status: implemented
(`dx status` diagnostics plus `dx version` pin/launcher/rollback dispatch).

There is no `dx doctor` command per
[ADR 0006](../decisions/0006-cli-command-surface.md). `dx status [--output text|json]`
reports toolchain resolution, platform/host coverage, missing tools, and pin staleness
with vocabulary `ok/warn/error` plus actionable hints; text prints one line per
check, JSON streams the NDJSON envelope (`command_started` plus one `status`
event per check plus `command_finished`, see the
[output protocol](../cli/output-protocol.md#status)). See the [hooks contract](../cli/commands/hooks.md)
for the two-layer hook configuration this surface reports on. Platform evidence beyond
Linux x86_64 remains a gap; no working multi-platform support is claimed until qualified
execution lands.

Per-repository `dx` version pinning uses a Bazelisk-style launcher
resolving the checked-in `.dx/version` pin (SemVer); `dx` version equals the pinned
`rules_dx` module version (`0.0.0`). The launcher refuses skew on every
workspace command at startup (fail on mutating/generating commands, warn
on read-only ones; see the
[startup skew gate](../cli/commands/status-version.md#startup-skew-gate));
`dx version --pin <ver>`
bumps from verified release artifacts only and `dx version --rollback` re-pins the
previous release. See
[distribution](../environments/environment.md#distribution).

## Authority Boundary

Accepted fit (2026-09-21, #963): the `DX_VERSION`/`MODULE_VERSION`
consts mirror `MODULE.bazel` at build time and the `.dx/version` pin is the
Bazelisk-style launcher pin, not Bazel graph semantics. The startup skew gate
stays one file read with no subprocesses by design. `dx status`
toolchain/platform/tools details report their `MODULE.bazel` and
`//quality/artifacts` sources statically with platform-evidence limits;
live Bazel resolution stays open and is never claimed here. The hook
per-check budget is CLI-owned timeout policy (see the
[hooks contract](../cli/commands/hooks.md#timeout-policy)), not Bazel
configuration.
