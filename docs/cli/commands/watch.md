# dx Watch

Implementation status: implemented.
Delivered: `dx watch` planning plus dispatch over the wrapped command with 200ms
debounce, `bazel-*`/`.dx`/`dx.local.toml` ignores, per-iteration re-resolution,
and local-only enforcement (refuses `CI=true`). Owning decision:
[ADR 0017](../../decisions/0017-dx-watch.md), extended to `check` and `fix` by
[ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md).
Platform scope inherits the [support matrix](../../product/support-matrix.md)
qualified hosts: the loop is portable Rust over `notify` 8.x plus
`notify-debouncer-mini` with no platform-specific execution path,
seed-host-delivered on Linux x86_64; unqualified hosts refuse with
`unsupported_platform` before any iteration. No separate per-platform watch
execution is claimed (wont-fix, issue #756).

```text
dx watch [--clear] <build|test|run|lint|typecheck|format|check|fix> [scope ...] [-- bazel-options ...]
```

`dx watch` loops one wrapped command. Every iteration reuses that command's
scope resolution, Bazel invocation, result collection, mutation, reporting,
and exit-status behavior verbatim. There is no daemon, cache, graph, remote
execution, or deployment mechanism.

- Scope defaults, file-to-owner mapping, and `//...` handling match the wrapped
  command. Scope is re-resolved each iteration.
- `--check`, `--output`, `--report`, `--fail-on`, `--quiet`, and `--dry-run`
  pass through to the wrapped command.
- `dx run` under `watch` reuses wrapped `run` semantics verbatim each
  iteration: file/directory scopes still enforce single-runnable selection
  (`ambiguous_runnable`/`no_runnable` are errors), while explicit labels and
  patterns run sequentially in scope order (see
  [dx run](build-test-coverage.md#dx-run)).
- Each iteration emits the wrapped command's normal stream starting with a
  fresh `command_started`. No cross-iteration state is exposed in machine output.
- Signals forward to the active child per the [CLI contract](../cli-contract.md#exit-status).
  CI use is not supported (wont-fix, issue #590): `dx watch` refuses
  `CI=true` as local-only, pinned by fixtures in
  `cli/cli/tests/fixtures/cli_execution_gaps/` plus `dx_adopt::plan_watch`.

## Execution Gaps

Decided under issue #590 (pinned by fixtures in
`cli/cli/tests/fixtures/cli_execution_gaps/` plus
`bazel run //tools/ci:cli_execution_gaps_qualification`; CLI-only, no
Bazel semantics change; seed only, no Supported claim):

- Watchable stays exactly `build`, `test`, `run`, `lint`, `typecheck`,
  `format`, `check`, and `fix` (8 commands). Every other registry command
  stays not watchable with fail-closed `not watchable` errors: `audit`,
  `bazel`, `bump`, `clean`, `codegen`, `completion`, `coverage`, `deps`,
  `deploy`, `docs`, `env`, `generate`, `hooks`, `init`, `migrate`, `owners`,
  `setup`, `status`, `update`, `version`, `watch` (no nesting), and `why`
  (22 commands). Silent substitution across commands stays rejected.
- Managed-state selection (`env`, `codegen`, `setup`, `clean`), lockfile
  mutation (`update`, `bump`, `migrate`), audit collection, BUILD-graph
  mutation (`generate`), one-shot adoption helpers (`init`, `hooks`,
  `status`, `version`, `owners`, `deps`, `why`, `completion`), the
  transparent `bazel` passthrough (no `dx` scope to re-resolve), heavy
  `coverage`, and single-deployable `deploy` have no re-resolvable
  file-iteration loop to reuse verbatim, so watching them stays wont-fix.
- CI refusal stays wont-fix: the loop never terminates, so CI must invoke
  the wrapped command once instead.
- Parallelism stays wont-fix: one iteration at a time with per-iteration
  re-resolution; no parallel iterations, caching, scheduling, or daemon.
