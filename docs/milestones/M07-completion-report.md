# M07 Completion Report: Initial Quality CLI And Full Initial Dogfood

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (command plans and workspace policy), WP2 (result projection and standard
reports), and WP3 (dogfood switch with direct-Bazel parity and coverage-gate
closure) are complete per the [M07 milestone](M07-initial-quality-cli-dogfood.md).
No silent deferrals. No capability-term transitions are claimed: `dx/cli` is
first-party implementation under test, not a product support claim.

## WP1: Plans And Workspace Policy

`dx/cli` (`dx_cli`) implements the `lint|typecheck|format` registry
(`args.rs`/`plan.rs`): canonical `--@rules_dx//config:workspace=//dx:config`
plus `--@rules_dx//config:validate=false`, required `--aspects`,
`--output_groups=dx_results`, `--keep_going`, and a per-run BEP path.
`--check` selects non-mutating mode; default mode mutates. No scope resolves
to `//...`; explicit `//`/`@` labels replace the repository scope and file
paths fail with `ScopeNotSupported` (exit 2). `main.rs` (`BinaryRunner`)
forwards `SIGINT`/`SIGTERM` to the Bazel child and honors absolute
`BUILD_WORKSPACE_DIRECTORY` under `bazel run` (getcwd resolves into the
execroot via the `bazel-bin` symlink otherwise).

## WP2: Projection And Standard Reports

`reports.rs` plans SARIF destinations (`-` owns stdout; text/diff summaries
are then suppressed and human findings move to stderr) and renders
deterministic SARIF 2.1.0 (per-tool runs, empty runs kept, partial runs carry
`invocations: [{executionSuccessful: false}]`). JSON mode emits the v1 NDJSON
lifecycle (`command_started`, per-findings `diagnostic`/`change`/`mutation`,
`command_finished` with `diagnostics`/`changes`/`mutations` counts);
`resolution` binds fixed/remaining/not_applied exactly to initial diagnostics
in mutating mode.

## WP3: Dogfood Switch, Parity, And Coverage Gate

CI job `corpus-dogfood` now runs `bazel run //dx/cli:dx -- lint|format|
typecheck $scope` in default mode plus `--check` no-op verification over the
29 `real_source_target(name = "corpus")` targets, replacing direct aspect
invocation. Parity evidence (this host, 2026-09-10):

- Direct lint aspects over the corpus scope: exit 0.
- `dx lint|format|typecheck` default and `--check`: exit 0, zero mutations,
  working tree unchanged (only pre-existing dirt remains).
- `dx lint --check --output=json --fail-on=error`: `command_started`,
  `command_finished` with `exit_code 0`, `results_complete true`,
  `diagnostics {info 0, warning 0, error 0}`, `changes {create 0, modify 0}`.
- Full gates: `bazel build //...` success; `bazel test //...` 58/58 pass
  (`dx_cli_test` 90+ unit tests behind `Runner`/`Fs` seams, no disk or
  subprocess use); `bazel coverage //...` with coverage gate
  **PASS 12612/12612** executable lines.

Stage order follows registry declaration (lint, typecheck, format);
typecheck is a valid no-op where no adapter applies. Convergence bound
`MAX_COMPLETED_ROUNDS = 10` held: every dogfood run converged with no
remaining changes.

Parity deviations (pre-existing, not dx defects): the
`fixture_real_markdown_no_config` fixture intentionally fails analysis;
direct-Bazel Clippy on `//quality/adapter:quality_adapter` reports
unresolved `serde` imports, a `real_aspects` limitation also present before
this milestone.
