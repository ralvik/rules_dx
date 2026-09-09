# ADR 0001: Bazel Owns Execution

## Status

Accepted.

## Context

The platform needs consistent local and CI checks with Bazel's dependency graph,
sandboxing, caching, and remote capabilities. A CLI that reconstructs actions or
executes checkers independently would create a second build system.

The repository currently contains no implementation that conflicts with this
decision.

## Decision

Bazel owns all non-mutating checks used by CI, including action creation, declared
inputs, tool selection, execution, caching, and result production. Such checks are
directly executable through Bazel. Analyzer actions complete successfully when a
tool ran and emitted a valid `dx_results` message, even when that message contains
findings. A Bazel-owned result evaluator or `dx` applies diagnostic failure policy;
tool, parser, protocol, and other infrastructure failures remain failed Bazel
actions.

`dx` may resolve scopes and compose Bazel invocations. It does not execute Ruff or
Ty directly for CI-style checks. `dx bazel` remains the transparent escape hatch.
Normal CLI output reports safe workflow summaries without subprocess argv or forwarded
option values. `--quiet` may suppress wrapper output; `--dry-run` and machine-readable
events preserve plan inspectability without rendering executable commands.

## Consequences

- Local and CI checks share target semantics.
- Rule/toolchain implementations must be hermetic and testable.
- The CLI remains small and checker-neutral.
- Direct Bazel CI evaluates the same normalized results and severity policy as
  `dx`, rather than relying on native tool exit codes.
- Bazel startup is part of workflow cost and must be measured rather than bypassed.
- Explicit source mutations require a separate policy under ADR 0005.

## Rejected Alternatives

- CLI-owned checker subprocesses for CI checks.
- A separate task graph, cache, dependency resolver, or lockfile.
- Moving Bazel semantics into hidden CLI logic that cannot be reproduced through the
  documented direct Bazel workflows. CLI output itself does not print argv.
