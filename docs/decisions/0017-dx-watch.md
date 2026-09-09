# ADR 0017: dx Watch Loop

## Status

Provisional.

## Context

`dx` is a transparent interface over Bazel; Bazel owns execution, caching,
and the source graph ([ADR 0001](./0001-bazel-owns-execution.md)).
[ADR 0006](./0006-cli-command-surface.md) freezes the initial
command surface without a watcher. The
[automatic-workflow preference](../product/scope.md#automatic-workflows)
forbids building a custom watcher, resolver, or refresh engine merely to make
a workflow automatic.

Upstream `ibazel` (`bazelbuild/bazel-watcher`) covers `build`/`test`/`run` on
explicit labels only. It cannot reuse `dx` file-to-owner resolution
([O44](../open-decisions.md)), all-direct-owners quality selection
([Target Resolution](../cli/target-resolution.md)), single-runnable enforcement
([O52](../open-decisions.md)), or `lint`/`typecheck`/`format` converge-and-apply
semantics ([Output Protocol](../cli/output-protocol.md)). Users requested an
inner loop for `build`, `test`, `run`, `lint`, `typecheck`, and `format`
including file scope.

## Decision

`dx watch` is a thin local-only loop around one existing `dx` command. It
reuses that command's scope resolution, Bazel invocation, result collection,
and mutation/report behavior verbatim on every iteration. It introduces no
graph, cache, resolver, daemon, remote execution, or deployment mechanism.

- Shape: `dx watch [--clear] <build|test|run|lint|typecheck|format> [scope ...] [-- bazel-options ...]`.
  `--check`, `--output`, `--report`, `--fail-on`, `--quiet`, and `--dry-run`
  pass through to the wrapped command.
- Scope: identical to the wrapped command, including file, directory, label,
  pattern, and `//...` default. Re-resolved every iteration; BUILD edits may
  change ownership between iterations.
- Out of scope: `coverage`, `audit`, `generate`, `env`, `codegen`, `setup`,
  `update`, `docs`, `init`, `hooks`, and `bazel`. No CI use.
- Each iteration emits the wrapped command's normal text/NDJSON stream starting
  with a fresh `command_started`. No cross-iteration state is exposed.
- Signals and TTY handling follow the [CLI contract](../cli/cli-contract.md#exit-status).
  Interruption terminates the active child before the watcher.
- Filesystem observation covers workspace source inputs only and ignores
  `bazel-*` outputs, `.dx` managed state, and ignored local overlays. Exact
  debounce, ignore set, and restart policy are qualified under O55.

## Consequences

- The [automatic-workflow preference](../product/scope.md#automatic-workflows)
  gains an explicit exception for this loop; upstream automation remains
  preferred where it preserves the required semantics.
- `dx run` under `watch` re-enforces single-runnable selection per iteration.
- Quality iterations reuse convergence, atomic apply, and `--fail-on` without
  new mutation semantics.
- `O44` target resolution must land before `watch` implementation.
- Tests cover scope kinds, pass-to-fail-to-recover cycles, ambiguous runnables,
  signal forwarding, and debounce behavior per O55.

## Rejected Alternatives

- `ibazel`-only interop with no `dx` loop: kept as documented interop for
  label-scoped `build`/`test`/`run`, but it cannot preserve file-scope
  resolution or quality pipeline semantics.
- Persistent daemon with cross-iteration cache or remote coordination:
  rejected as a second execution graph outside Bazel ownership.
- Generic `watchexec`/`entr` plus `dx`: rejected as the default because it
  retriggers without the Bazel watch graph and thrashes on outputs.
