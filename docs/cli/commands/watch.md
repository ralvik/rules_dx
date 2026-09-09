# dx Watch

Provisional contract. Do not implement until [O44](../../open-decisions.md) selects
the target-resolution query and [O55](../../open-decisions.md) freezes debounce,
ignore set, restart, framing, and local-only semantics. Owning decision:
[ADR 0017](../../decisions/0017-dx-watch.md), extended to `check` and `fix` by
[ADR 0018](../../decisions/0018-umbrella-check-fix-cleanup-clean.md) subject to O55.

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
- `dx run` under `watch` enforces single-runnable selection per iteration
  (`ambiguous_runnable`/`no_runnable` are errors; see [O52](../../open-decisions.md)).
- Each iteration emits the wrapped command's normal stream starting with a
  fresh `command_started`. No cross-iteration state is exposed in machine output.
- Signals forward to the active child per the [CLI contract](../cli-contract.md#exit-status).
  `coverage`, `audit`, `generate`, `env`, `codegen`, `setup`, `update`, `docs`,
  `init`, `hooks`, and `bazel` are not watchable. CI use is not supported.
