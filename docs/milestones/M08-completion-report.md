# M08 Completion Report: Target Resolution And Basic Commands

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (label/pattern/path/dir/owner resolution), WP2 (test/coverage
reverse-dependency mapping and error taxonomy), WP3 (build/test/coverage
wiring with standard reports), and WP4 (O52 qualification plus `dx run`)
are complete per the [M08 milestone](M08-target-resolution-basic-commands.md).
No silent deferrals. No capability-term transitions are claimed: `dx/cli` is
first-party implementation under test, not a product support claim.

## WP1: Resolver

`dx/cli/src/resolve.rs` implements Bazel-backed scope resolution with no
BUILD parsing and no filesystem source enumeration. Main-workspace labels
and patterns pass through after syntax/workspace validation; workflow
commands reject external-repository (`@`) scopes. Files resolve to a
`//pkg:rel/path` label via nearest-enclosing-package walk (existence probe
only); directories map to `//path/...` recursion owned by Bazel.
Ownership of one file is exactly
`kind('rule', rdeps(//..., <file-label>, 1))` at depth 1 over `//...`, one
query per file; every depth-1 rule referrer (including filegroups) is a
direct owner, canonicalized, deduplicated, bytewise sorted. Non-package
directories fail as not-a-package, workspace-missing paths as not-found,
query-visible non-source paths as generated-excluded. This selects
[O44](../open-decisions.md) (unconfigured `bazel query` only; no cquery, no
aspect; `select()` over-selection accepted as conservative; warm-server
single-file query ~0.1s).

## WP2: Test/Coverage Mapping

`map_owners_to_tests` runs
`kind('.*_test rule', rdeps(//..., set(<owners>)))`; empty mappings are
explicit `NoTests` errors (exit 2), never silent success and never a direct
library passed to `bazel test`/`coverage`. `dx/cli/src/plan.rs`
(`WorkflowVerb::of`) exposes the single command-definition source consumed
by O61 completion generation.

## WP3: Build/Test/Coverage

`execute_workflow`/`execute_test_reports` plan
`bazel + WORKFLOW_STARTUP_OPTS + <verb> + workspace flag + targets`.
`test` renders deterministic JUnit XML (one `<testsuites name="dx">`, one
`<testsuite name="//pkg:target">` per target, bytewise order); `coverage`
requires `--combined_report=lcov` and validates LCOV derived from BEP
`coverage.dat`/`test.lcov` artifacts. `build`/`run` collect no reports;
Bazel exit codes pass through verbatim. `dx/bep` collects
`TestOutputFile{label,name,exec_path,run,shard,attempt}` with 1-based
shard/attempt parsing.

## WP4: Run (O52 Qualified)

[O52](../open-decisions.md) qualified in WP0; `dx run` implements the
[command contract](../cli/commands/build-test-coverage.md#dx-run): labels
and patterns pass through unchanged (Bazel owns alias/executability); file
and directory scopes resolve through the shared owner query then require
exactly one runnable depth-1 owner (rule kind ends in `_binary`, aliases
not followed): zero fails `no_runnable`, multiple fails
`ambiguous_runnable` with sorted candidates (both operational, exit 1).
App exit code is preserved verbatim; scope/usage failures stay pre-exec
(exit 2). App inherits stdio with SIGINT/SIGTERM forwarding via
`CHILD_PID`; args after `--` forward verbatim. Only `--output=text`
accepted; no `--report`; refuses when `CI=true` (local-only). Multirun
stays out of scope.

## Evidence

Exact commands on this host (2026-09-10):

- `bazel build //...`: success, 181 targets.
- `bazel test //...`: 58/58 pass (`dx_cli_test` 192 unit tests behind
  `Runner`/`Fs`/`QueryRunner` seams, no disk or subprocess use).
- `bazel coverage //...` plus
  `bazel run //tools/coverage:check -- --report
  $PWD/bazel-out/_coverage/_coverage_report.dat --inventory
  $PWD/tools/coverage/inventory.txt --sources /tmp/m08_sources.txt
  --root $PWD`: **PASS 16073/16073** executable lines across 36 Rust
  source files (34 eligible inventory entries), including
  `exec.rs 2611/2611 (55 ignored)`, `reports.rs 1403/1403 (5 ignored)`,
  `resolve.rs 1130/1130 (16 ignored)`, `plan.rs 413/413`,
  `args.rs 517/517`, `bep 920/920`.
- `bazel test //dx/cli:dx_cli_fmt_test //dx/cli:dx_cli_clippy_test
  //dx/cli:dx_cli_test`: 3 pass; rustfmt via the Bazel toolchain config,
  Clippy with warnings as errors.

Fixtures prove no BUILD content reads (existence probes only), every owner
preserved, ambiguity/ownership/parity cases, run scope passthrough,
single-runnable selection, zero/ambiguous failures, `--` forwarding,
TTY/signal forwarding, and local-only refusal. Planned invocations match
direct Bazel for success, failure, and empty mappings.

## Changed Components

New `dx/cli/src/resolve.rs` (+ `eligible` inventory entry); extended
`dx/cli/src/{args,exec,plan,reports,lib,main}.rs`, `dx/bep/src/lib.rs`,
`dx/cli/{BUILD.bazel,Cargo.toml}` (`quick-xml 0.36`). Docs:
`cli/target-resolution.md` (O44 strategy), `cli/commands/build-test-coverage.md`
(O52 run contract), `cli/commands/quality.md`, `open-decisions.md`
(O44 selected, O52 qualified). `MODULE.bazel.lock`/`rust/hello/Cargo.lock`
regenerated.

## Deviations, Gaps, Exclusions

No deviations from the milestone scope. Out of scope respected: generate,
env, codegen, setup, audit, update, multirun, language semantics untouched.
Gaps: non-Linux platforms, remote/cache qualification, and external-consumer
evidence remain for M27+; cquery stays deferred unless measured
over-selection justifies it.
