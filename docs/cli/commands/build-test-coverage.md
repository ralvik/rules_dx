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

Status: scope and forwarding qualified under
[O52](../../open-decisions.md); implementation may proceed against this
section. Strict single-target execution applies to file/directory
resolution scopes only — labels and target patterns pass through to
`bazel run` unchanged (Bazel owns alias and executability). A file or
directory scope resolves through the same ownership query as `dx build`,
then requires exactly one runnable owner, where runnable means a
depth-1 owner whose rule kind ends in `_binary` (aliases are not
followed for file scopes): zero runnables fail pre-resolution with
`no_runnable`, multiple with `ambiguous_runnable` listing sorted
candidates; both are operational failures (exit 1). The application
exit code is preserved verbatim (success included); scope/usage
failures stay pre-exec (exit 2). The application inherits stdio with
SIGINT/SIGTERM forwarding; arguments after `--` forward verbatim.
`dx run` accepts only `--output=text` (json/diff rejected pre-exec),
emits prose lifecycle on stderr, takes no `--report`, and refuses when
env `CI=true` (local-only). Multirun remains a separate future
feature.

## `dx coverage`

`dx coverage` resolves scope and invokes Bazel coverage. Bazel owns instrumentation,
test execution, and coverage artifacts. File scope uses the exact same all-owner,
transitive reverse-dependent test mapping and empty-result behavior as `dx test`.
Explicit test labels and patterns bypass inference. LCOV is the initial stable report
format and uses the shared `--report` stdout-or-file destination contract. Additional
formats require an authoritative adapter or a later compatibility decision. Because
LCOV has no portable partial marker, failed collection is signaled by command status and
the live NDJSON or stderr diagnostic while available validated records remain valid LCOV.
