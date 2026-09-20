# dx Watch

Implementation status: implemented.
Delivered: `dx watch` planning plus dispatch over the wrapped command with 200ms
debounce, `bazel-*`/`.dx`/`dx.local.toml` ignores, per-iteration re-resolution,
and local-only enforcement (refuses `CI=true`). Owning decision:
[ADR 0017](../../decisions/0017-dx-watch.md), extended to `check` and `fix` by
[ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md).
Platform evidence beyond Linux x86_64 remains a gap; no working multi-platform
support is claimed until qualified execution lands.

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
  `coverage`, `audit`, `generate`, `env`, `codegen`, `setup`, `update`,
  `init`, `hooks`, and `bazel` are not watchable. CI use is not supported.
