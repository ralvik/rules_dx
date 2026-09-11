# M10 Completion Report: Public Generate, Config Binding, And Result Transport

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (O48 scoped selection plus versioned exact records in one Gazelle run),
WP2 (CLI projection through check, default, diff, text, and NDJSON modes),
WP3 (selected native-config targets, closures, visibility, ownership, and
direct hints), and WP4 (O59 umbrellas with stop-on-first-failure and phase
exit-code preservation) are complete per the
[M10 milestone](M10-public-generate-config-binding-result-transport.md). No
silent deferrals. No capability-term transitions are claimed: generation is
first-party implementation under test, not a product support claim.

## WP1: Scoped Planning, Intended Witness, Finalizer, Dispatch

- `dx/cli/src/plan.rs`: scoped planning from resolved scope (O48 reuses O44
  target resolution). `GENERATE_TARGET` (`//dx:generate`) and
  `GENERATE_CHECK_TARGET` (`//dx:generate_check`, upstream `-mode diff`
  encoded in canonical-target wiring), `generate_traversal_dirs`,
  `generate_scope_json` (`DX_GENERATE_SCOPE`), `intended_path`, and the
  `DX_GENERATE_INTENDED` / `DX_GENERATE_SCOPE` / `DX_GENERATE_MODE`
  constants. Empty scope stays repo-wide (`//...`); scoped check freshness
  ends at the resolved package-set edge.
- `gazelle/rust/manifest.go`: the extension witnesses its exact BUILD
  changes as JSON on `DX_GENERATE_INTENDED` (candidate bytes, original
  bytes, byte-offset edits, all base64) from `AfterResolvingDeps`, replaying
  `merger.FixLoads` plus `Format` so the witness converges with what Gazelle
  computes. Ignored imports sort by (path, language, import) before
  encoding, per the crate order rule.
- `generation/result` (key `generation/generation_result`): versioned
  transport crate (`SCHEMA_MAJOR = 1`, `SCHEMA_MINOR = 0`), `validate`,
  `candidate`, `digest`, `encode_validated` / `decode_validated`.
- `dx/cli/src/finalize.rs`: the finalizer turns the witness into the
  validated manifest with finalizer-owned BLAKE3 `original_digest` (O13 —
  the CLI never inspects the workspace except guarded outcome reads).
  Default mode compares workspace bytes per file (`applied` /
  `not_applied` with `write_mismatch` / `missing_file` /
  `unreadable_file`); check mode reads no files and stamps `unspecified`.
  A complete check witness finalizes even when Gazelle exits nonzero:
  upstream writes the witness in `AfterResolvingDeps` before the emit loop
  (`v2/cmd/gazelle/update/update.go`), and without `-patch` the only
  post-witness failure is `ErrDiff` ("changes found"), which exits 1
  silently (`v2/cmd/gazelle/main.go`) — the expected check signal. Only a
  missing or incomplete witness degrades to the incomplete envelope.
- `dx/cli/src/exec.rs` `execute_generate`: sets the private protocol
  environment on the runner (the shared `Runner` trait carries per-command
  env), preserves Gazelle's exit status through the projection, fails
  closed (`invalid_result`) on a missing or contradictory witness after a
  successful run, and renders text, diff, and NDJSON from the manifest
  without rerunning Gazelle. `//dx:generate_check` shares the language
  wiring, so direct Gazelle and `dx generate` compute identical rewrites
  through the same `//gazelle/rust:gazelle` binary.

## WP2: Projection

`dx/cli/src/generate.rs`: pure projection of the validated manifest —
per-file changes (create/modify with exact edits and digests), default-mode
mutations with failure reasons, ignored-import notices
(`IGNORED_IMPORT_CODE`), `FinishedCounts`, and exit selection (nonzero
Bazel code preserved; incomplete fails; check fails on any change; default
fails on any `not_applied`). Covered 537/537 with zero exclusions.

## WP3: Native-Config Targets And `aspect_hints`

`gazelle/rust/native_config.go` (landed ahead of this report; see the M11
report's inventory note): selected-tool native-config target generation,
closures, visibility, ownership, and direct `aspect_hints` binding with
list-merge semantics — Gazelle owns only `rules_dx` entries, foreign and
hand entries are preserved, removals and skips behave. Proven by
`native_config_test.go` (17 tests), the `native_config_test.sh` golden run
of the built Gazelle binary over `testdata/native_config`, and the
`quality/testdata` binding-shape fixtures.

## WP4: `dx check` / `dx fix` Umbrellas

`Command::Check` / `Command::Fix` (`args.rs`), SARIF-only specs
(`plan.rs`), and `execute_umbrella` (`exec.rs`): sequential `format` →
`lint` → `typecheck` → `generate` (check mode under `check`, default under
`fix`, explicit `--check` forcing check mode under `fix`), each phase
reusing its wrapped command verbatim with captured streams and derived
nonces so BEP streams and intended manifests never alias. First nonzero
phase stops the umbrella with its exit code; one parent
`command_started` / `command_finished` pair brackets the verbatim
per-phase streams; `--output diff` concatenates validated phase patches;
each SARIF request merges the executed SARIF-capable phases' `runs` in
phase order; stdout destinations fail closed pre-execution. Sixteen
`umbrella_*` / `check_*` / `fix_*` tests cover order, short-circuit,
lifecycle bracketing, SARIF merge and stop-prefix content, diff
concatenation, stdout/unknown-format rejection, and report write failure.

## Public Surface And Load Labels

- Canonical targets: `//dx:generate`, `//dx:generate_check`.
- CLI: `dx generate [scope ...] [--check] [--output=text|json|diff]`,
  `dx check [scope ...]`, `dx fix [scope ...]` with `--report`, `--fail-on`,
  `--quiet`, `--dry-run` passthrough.
- Private protocol (extension↔finalizer only):
  `DX_GENERATE_INTENDED` / `DX_GENERATE_SCOPE` / `DX_GENERATE_MODE`;
  manifest schema v1 via `generation/generation_result`.

## Evidence

Exact commands on this host (2026-09-11), run against the committed tree:

- `bazel build //...`: success (241 targets).
- `bazel test //...`: 77/77 pass, including `//dx/cli:dx_cli_test`
  (finalizer: 13 tests — strict base64 vectors, outcome matrix,
  unsafe-path guard, `ErrDiff` trust rule; projection: 9 tests; dispatch:
  generate/check/diff/umbrella suites), `//gazelle/rust:rust_test`
  (intended-manifest recorder suite including ignored-import sort order),
  and `//generation/result` transport tests (9 tests).
- `bazel coverage //...`: 72/72 pass.
- Coverage gate (`//tools/coverage:check` over the LCOV report, full
  Bazel-declared `*.rs`/`*.go` sources vs `tools/coverage/inventory.txt`):
  **21806/21806** executable lines covered, including
  `dx/cli/src/finalize.rs: 478/479`, `dx/cli/src/exec.rs: 3495/3596`,
  `dx/cli/src/generate.rs: 537/537`, `dx/cli/src/plan.rs: 621/621`,
  `dx/cli/src/args.rs: 543/543`, `gazelle/rust/manifest.go: 215/220`,
  `gazelle/rust/native_config.go: 242/244`, and
  `generation/result/src/lib.rs: 570/578` (remainders are `LCOV_EXCL`
  defense-in-depth arms with recorded reasons, per repo precedent).
- `dx_cli_fmt_test` / `dx_cli_clippy_test`: rustfmt via the pinned
  toolchain config, Clippy with warnings as errors (fresh
  `dx_cli.rustfmt.ok` / `dx_cli.clippy.ok` stamps regenerated on the
  verifying build).
- `dx generate --check` production path is pinned by
  `generate_check_reports_changes_despite_diff_exit` (changes witness
  with Bazel code 1 → change events, no mutations, exit 1) and
  `generate_incomplete_check_reports_nothing` (incomplete witness with
  code 2 → no events, `results_complete: false`, code 2).
- Umbrella composition fixtures (in `dx_cli_test`): SARIF run order,
  stop-prefix content, stdout-destination rejection, unsupported-format
  rejection, and multi-report independence, per the
  [check/fix/clean contract](../cli/commands/check-fix-clean.md).
- O48 evidence: resolver-mapping, scope-edge freshness, attribution, and
  narrowed-merge fixtures in `dx_cli_test` (`generate_plan_*`,
  `generate_scope_json_*`, `generate_dispatch_env_*`); O59 evidence: the
  umbrella suite above. O13 evidence: the transport crate plus
  finalizer/dispatch tests.

## Changed Components

- `dx/cli/` (`args.rs`, `exec.rs`, `finalize.rs`, `generate.rs`,
  `lib.rs`, `main.rs`, `plan.rs`, `BUILD.bazel`, `Cargo.toml`): planning,
  finalizer, projection, dispatch, umbrellas, wiring.
- `dx/process/src/lib.rs`: `Runner::run` carries per-command environment.
- `dx/BUILD.bazel`: canonical `//dx:generate` and `//dx:generate_check`.
- `gazelle/rust/` (`manifest.go`, `manifest_test.go`, `lang.go`,
  `BUILD.bazel`): intended-manifest recorder and sorted ignored imports.
- `generation/` (`result.proto`, `result/` crate, `BUILD.bazel`):
  versioned manifest transport.
- `tools/coverage/inventory.txt`: new eligible sources.
- `docs/cli/commands/generate.md`, `docs/open-decisions.md` (O13 dispatch
  landing): contract updates.
- This report; milestone index report link.

## Deviations, Gaps, Exclusions

No deviations from the milestone scope. Out of scope respected:
non-Rust application generation, codegen execution, environment
preparation, and Rust BUILD parsing in the CLI. Gaps: non-Linux host
evidence, remote/cache qualification, and external-consumer evidence
(M27+). Remaining Rust mappings stay with M12 under O24.
