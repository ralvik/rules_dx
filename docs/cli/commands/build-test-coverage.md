# Build, Test, And Coverage Commands

## `dx build` And `dx test`

Accept labels, target patterns, paths, or no scope. Labels and target patterns
map directly to Bazel. Build paths resolve to all direct source owners. Test file
paths resolve owners, then select every test in
`tests(rdeps(//..., set(<owner labels>)))`. An empty mapped test set is an error with
guidance to pass an explicit test label or pattern. Paths otherwise follow
[Target Resolution](../target-resolution.md). No-scope build runs
`bazel build //...`, and no-scope test runs `bazel test //...`; users who need a
smaller invocation provide a path, label, or target pattern explicitly, or pass
`--here` (`--cwd` alias) for the current directory tree (`//path/...`; `//...`
at the root).

`dx test` supports JUnit XML reports through the shared `--report` contract. Bazel test
results remain authoritative; `dx` normalizes their reported result artifacts into one
requested document without changing test execution or status. `dx build` has no
additional standard report format.

## `dx run`

Status: implemented as specified in this
section (pinned by `dx_cli` run plus `resolve_run` fixtures, delivered under
issue #463). Strict single-target execution applies to file/directory
resolution scopes only. Multiple explicit labels run sequentially in
scope order (`dx run //demo:frontend //demo:backend`), each as its own
`bazel run` with the same arguments after `--` forwarded to every
target; explicit target patterns containing `...` or `*`
(`dx run //demo/...`) expand Bazel-owned through one
`kind('.*_binary rule', <pattern>)` query each (each expansion sorted,
concatenated in input order with first-seen dedup across labels and
patterns), then run sequentially
in input order. A file or
directory scope resolves through the same ownership query as `dx build`,
then requires exactly one runnable owner, where runnable means a
depth-1 owner whose rule kind ends in `_binary` (aliases are not
followed for file scopes): zero runnables fail pre-resolution with
`no_runnable`, multiple with `ambiguous_runnable` listing sorted
candidates; both are operational failures (exit 1), and an explicit
pattern expanding to nothing is `no_runnable`. The application
exit code is preserved verbatim (success included); sequential multirun
stops on the first required failure and returns that code, per the
[CLI contract](../cli-contract.md#exit-status). Scope/usage
failures stay pre-exec (exit 2). Each application inherits stdio with
SIGINT/SIGTERM forwarding to the active child (sequential mode never
has more than one live child, so no supervisor table); arguments after
`--` forward verbatim.
`dx run` accepts `--output=text|json` (`--output=diff` has no patch to emit and is
rejected pre-exec). Text emits one prose lifecycle line per target on stderr; JSON
streams `command_started`, one `execute` `operation` per target with its single-label
scope in execution order, an optional sanitized `bazel_failed` `error` naming the failed
target, and `command_finished` on stdout with child output routed to stderr so stdout
stays machine-owned. It takes no `--report`, and refuses when
env `CI=true` (local-only).

## `dx deploy`

Status: implemented as specified in this section. Strict single-label
execution: exactly one main-workspace label (`//pkg:target`); patterns
(`//...`), multiple labels, and file/path scopes are usage failures
(exit 2). Deployability is checked with one `bazel cquery` before any
build: the target must return `DxDeployInfo` (see
[Deploy authoring](../../deploy/authoring.md)) or be executable
(`*_binary`/executable; aliases included, Bazel owns executability),
else `not_deployable` fails pre-exec (exit 2). The flow is resolve,
cquery, `bazel build --config=...`, then `bazel run --config=...` with
`DX_PROFILE=debug|dev|release` forwarded to the deploy program.
`--dry-run` prints the plan (label, app, profile, build/run commands)
and executes nothing. Arguments after `--` forward verbatim to the
program. Exit codes preserve Bazel/program status verbatim; stdio is
inherited with SIGINT/SIGTERM forwarding, like `dx run`. Like `dx run`,
only `--output=text`, prose on stderr, no `--report`.

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
markers (with a nearby `reason:` comment, line comments outside string literals only; block comments and raw strings stay wont-fix per issue #589, see [Testing Strategy](../../testing/README.md#coverage)) exclude lines from the denominator;
sources that fail to load and non-Rust/Go records count raw. Without the flag,
coverage collects and reports with no threshold verdict. The threshold is a
configurable requirement for users per-cell: each required
configuration/platform cell gates its own report against its own
`--min-coverage` value (this repository pins `97` for the `//...` rate
gate with a zero-uncovered exact gate over the versioned inventory
scope). Per-cell plus Codecov opt-in plus remote evidence is qualified
with fixture evidence pinned in
`tools/coverage/tests/fixtures/per_cell/pins.bzl` via
`bazel run //tools/ci:coverage_qualification` (issue #507).

Coverage tools resolve per-host via the registered C++ toolchain. The
vendored preset pins only `GENERATE_LLVM_LCOV=1` (LLVM LCOV where the
toolchain emits profraw) plus `combined_report=lcov`, never an ambient
`COVERAGE_GCOV_PATH=/usr/bin/*` path. `COVERAGE_GCOV_PATH`/`LLVM_COV`/
`LLVM_PROFDATA` come from `cc_toolchain` per host; a GCC-gcov-only path
where the toolchain provides no gcov fails closed in Bazel's collection
script rather than silently using a wrong host path.

## Build Profiles

Three shared Bazel configs select `compilation_mode` behind stable
`dx_*` names (see [ADR 0021](../../decisions/0021-build-profiles.md)):

| Config | `compilation_mode` | Intended default |
| --- | --- | --- |
| `dx_debug` | `dbg` | Diagnostics |
| `dx_dev` | `fastbuild` | `build`/`run`/`test` default; equals bare-invocation behavior |
| `dx_release` | `opt` | `deploy` default |

The configs live in the vendored preset (`tools/bazelrc/preset.bazelrc`,
reviewed via `tools/bazelrc/src/lib.rs`, wired through the root
`.bazelrc`) and are additive: bare invocations keep today's behavior.

`dx build`, `dx run`, `dx test`, and `dx deploy` accept
`--debug`/`--release` to
select the profile. The flags are mutually exclusive (both passed is a
usage failure, exit 2) and map to `--config=dx_debug`/`--config=dx_release`
on the Bazel argv; the bare invocation passes `--config=dx_dev`
explicitly, except `dx deploy` which passes `--config=dx_release`.
There is no `--dev` flag: bare already means the middle
mode. `dx coverage` takes no profile flags; its argv is unchanged.
Precedence is explicit flag over deploy target `profile` attribute over
command default. The deploy target `profile` attribute comes from
`DxDeployInfo` (see [Deploy authoring](../../deploy/authoring.md));
`DX_PROFILE=debug|dev|release` is forwarded to the deploy program.
Flags plus forwarding are pinned by
`cli/cli/tests/fixtures/build_profiles/` (`pins.bzl` plus
`build_profiles.expected`) via
`bazel run //tools/ci:build_profiles_qualification`
(qualified seed-only under issue #814 with no Supported claim).
