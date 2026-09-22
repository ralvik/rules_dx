# Inspect Wrappers (`dx owners`, `dx deps`, `dx why`)

Implementation status: implemented.
Thin `bazel query`/`cquery` forwarding reusing
[target resolution](../target-resolution.md) with no custom graph engine.

## Scope forms

`dx owners <scope>...` and `dx deps <scope>...` accept one or more explicit
scopes; each scope runs one query and results merge. `dx why <file> <label>`
takes exactly one file scope plus one target label: it resolves the file's
depth-1 owner first, then explains one path from that owner to the label with
`somepath`. Empty scopes and external (`@repo//...`) scopes fail pre-exec
(exit 2); `why` with anything other than two positionals fails the same way.

## `--configured`

By default the wrappers run `bazel query`. With `--configured` they run
`bazel cquery` instead (`deps`, `owners`, and the `somepath` leg of `why`).
The flag belongs to these three commands only; every other command rejects
it pre-exec. It is distinct from `dx clean --bazel` (which forwards
`bazel clean` after pruning) and from the `dx bazel` passthrough command
(which forwards raw args): see `dx <command> --help` per-command flags.

## Output and exit codes

Stdout is bytewise-sorted deduplicated canonical labels, one per line.
Failures are operational (exit 1): `query failed` when Bazel fails,
`no owner for <file>` when `why` finds no depth-1 owner. Usage errors
(bad scope, wrong arity, unsupported flags) exit `2`.

`--output=json` reuses the status envelope (see the
[output protocol](../output-protocol.md#status)): `command_started`, then one
`status` event per label (`name` is the command, `detail` is the label,
`hint` is the requesting scope, or `<file> -> <label>` for `why`), then an
optional `error` (`bazel_failed` with `phase: query`, `no_owner`, or
`invalid_result`), then `command_finished` with only `exit_code`.
`--output=diff` is rejected pre-exec (exit 2).

Expressions are `kind('rule', rdeps(//..., <scope>, 1))` for `owners`,
`deps(<scope>)` for `deps`, and `somepath(<owner>, <label>)` for `why`;
the verb and expression stay separate argv elements and are never
double-wrapped.

`--dry-run` plans without launching Bazel: `owners`/`deps` print
`would run bazel <verb> <expr>` per scope, `why` prints
`would run bazel <verb> <expr> then somepath to <label>`. Plans are
summaries, suppressed under `--quiet`.
