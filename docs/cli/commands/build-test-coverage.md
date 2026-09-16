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

Status: implemented as specified in this
section (pinned by `dx_cli` run fixtures). Strict single-target execution applies to file/directory
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

`dx coverage --min-coverage <percent>` additionally enforces a line-coverage
threshold over the collected LCOV: covered over eligible executable lines must
reach the integer percent, else the command exits 1. `LCOV_EXCL_*` source
markers (with a nearby `reason:` comment) exclude lines from the denominator;
sources that fail to load and non-Rust/Go records count raw. Without the flag,
coverage collects and reports with no threshold verdict.

## Build Profiles

Three shared Bazel configs select `compilation_mode` behind stable
`dx_*` names (see [ADR 0021](../../decisions/0021-build-profiles.md)):

| Config | `compilation_mode` | Intended default |
| --- | --- | --- |
| `dx_debug` | `dbg` | Diagnostics |
| `dx_dev` | `fastbuild` | `build`/`run`/`test` default; equals bare-invocation behavior |
| `dx_release` | `opt` | `deploy` default |

The configs live in the vendored preset (`tools/bazelrc/preset.bazelrc`,
reviewed via `tools/bazelrc/preset.py`, wired through the root
`.bazelrc`) and are additive: bare invocations keep today's behavior.

`dx build`, `dx run`, and `dx test` accept `--debug`/`--release` to
select the profile. The flags are mutually exclusive (both passed is a
usage failure, exit 2) and map to `--config=dx_debug`/`--config=dx_release`
on the Bazel argv; the bare invocation passes `--config=dx_dev`
explicitly. There is no `--dev` flag: bare already means the middle
mode. `dx coverage` takes no profile flags; its argv is unchanged.
Precedence is explicit flag over deploy target `profile` attribute over
command default. The `deploy` default (`release`), the target attribute
(open in [#178](https://github.com/ralvik/rules_dx/issues/178)), the
`dx deploy` flag surface, and `DX_PROFILE=debug|dev|release`
forwarding to the deploy program are provisional until
[#180](https://github.com/ralvik/rules_dx/issues/180) lands.
