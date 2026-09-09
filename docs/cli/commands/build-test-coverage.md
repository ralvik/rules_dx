# Build, Test, And Coverage Commands

## `dx build` And `dx test`

Accept labels, target patterns, paths, or no scope. Labels and target patterns
map directly to Bazel. Build paths resolve to all direct source owners. Test file
paths resolve owners, then select every test in
`tests(rdeps(//..., set(<owner labels>)))`. An empty mapped test set is an error with
guidance to pass an explicit test label or pattern. Paths otherwise follow
[Target Resolution](../target-resolution.md). No-scope build runs
`bazel build //...`, and no-scope test runs `bazel test //...`; users who need a
smaller invocation provide a path, label, or target pattern explicitly.

`dx test` supports JUnit XML reports through the shared `--report` contract. Bazel test
results remain authoritative; `dx` normalizes their reported result artifacts into one
requested document without changing test execution or status. `dx build` has no
additional standard report format.

## `dx run`

Status: target shape accepted under
[O52](../../open-decisions.md); disambiguation, exit codes, output-protocol
events, and CI exclusion are pending. Strict single-target execution — a file scope resolves to its owning
library plus thin binary wrapper per [generation ownership](../../generation/common.md#executable-entries),
never arbitrary reverse-dependency execution. Do not implement against this prose until
that qualification lands.

`dx run` accepts labels, target patterns, files, or directories. Labels and
target patterns pass through to Bazel unchanged. File and directory scopes
resolve through the same ownership query as `dx build`, then require exactly
one runnable owner: zero runnables fail with `no_runnable`, multiple runnables
fail with `ambiguous_runnable`. Arguments after `--` forward to the application
binary. Execution is local-only with TTY and signal forwarding; CI use is not
supported. Multirun remains a separate future feature.

## `dx coverage`

`dx coverage` resolves scope and invokes Bazel coverage. Bazel owns instrumentation,
test execution, and coverage artifacts. File scope uses the exact same all-owner,
transitive reverse-dependent test mapping and empty-result behavior as `dx test`.
Explicit test labels and patterns bypass inference. LCOV is the initial stable report
format and uses the shared `--report` stdout-or-file destination contract. Additional
formats require an authoritative adapter or a later compatibility decision. Because
LCOV has no portable partial marker, failed collection is signaled by command status and
the live NDJSON or stderr diagnostic while available validated records remain valid LCOV.
