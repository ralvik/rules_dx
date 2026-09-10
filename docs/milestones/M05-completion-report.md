# M05 Completion Report: Direct Bazel Dogfood (WP1-WP4)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

Capability-term transitions for the adapters stay as recorded in the
[M04 completion report](M04-completion-report.md) until M05 exits; the
evidence below closes the M05 exit gates.

## Scope Completed And Deferred

WP1 (corpus ownership) and WP2 (adapters over corpus) are complete per the
[M05 milestone](M05-direct-bazel-dogfood.md). WP3 (CI enforcement plus
action/cache measurements) is complete except the preset-dependent CI
extension, which belongs to WP4. No silent deferrals.

## WP4: Vendored .bazelrc Preset

`tools/bazelrc` adopts the vendored execution preset (O62). The reviewed
inventory lives in `tools/bazelrc/preset.py`; the generator renders two
checked-in files: `tools/bazelrc/bazelrc-preset.bzl` (Starlark data:
`PRESET_BAZEL_VERSION`, `PRESET_FLAGS`, `EXTRA_PRESETS`) and
`tools/bazelrc/preset.bazelrc` (the `.bazelrc` fragment the root
`.bazelrc` imports). Upstream flag recommendations enter only as reviewed
inventory edits; the generator never fetches. Root `.bazelrc` keeps project
overrides explicit (none today beyond the preset) and `user.bazelrc` last.

The preset pin is the Bazel version: `PRESET_BAZEL_VERSION = "9.2.0"`
matches `.bazelversion` (latest stable per
[dependency currency](../decisions/0008-dependency-currency.md); 9.2.1,
9.3.0, and 9.4.0 were verified absent). `preset.update_test`
(`//tools/bazelrc:preset.update_test`, in `bazel test //...`) pins the pin,
the flag counts, and the rendered substrings hermetically, so a version
bump without a reviewed regen fails the suite. `preset.update
--verify-only` additionally rejects stale files with a printed flag diff
and rejects root `.bazelrc` lines duplicating preset flags (reconcile by
removing owned duplicates).

Reviewed inventory (6 flags, zero behavior change from the M00 seed
`.bazelrc`): `common --enable_bzlmod` (Bzlmod is the only supported
resolution; explicit against future default flips),
`build --verbose_failures` (action diagnostics need no second fetch),
`test --test_output=errors` (reviewable logs), plus the owned
`extra_presets` group `coverage` (`GENERATE_LLVM_LCOV=1` for the rules_rust
collect_coverage step, `combined_report=lcov`, `instrumentation_filter=^//`
scoped by the source inventory). Adoption moved the M00 lines byte-identical
into the fragment: warm `bazel build //...` stays fully cache-hit and
`bazel coverage` still emits the combined report.

Dependency-update automation (O53 interim): `renovate.json` enables
Renovate for the `bazel` manager over the `.bazelversion` pin surface with
reviewable PRs and no auto-merge. Regen, flag-diff review, test-pin
updates, and full verification stay manual until O53 qualifies bot-owned
regen. The update-loop proof (drill PR exercising skew, red test, regen,
review, verification, and manual merge) is recorded in the loop-evidence
note below; consumer preset delivery stays M27 scope.

O-item resolutions: O21 is resolved (adapters landed directly with
fixtures and no seed checks; the rule stands in
[Testing](../testing/README.md) and the M13 cleanup remains M13 scope).
O62 is resolved (execution flags are execution policy per
[ADR 0011](../decisions/0011-configuration-composition.md); preset
vendored with per-flag review and owned `extra_presets`). O53 stays open
for the native bot; the Renovate interim above is recorded in its row.

### Update-Loop Drill Evidence

Branch `drill/preset-update-loop` exercised the full loop against a
simulated bump. No newer stable Bazel exists (9.2.1, 9.3.0, 9.4.0 verified
absent through Bazelisk: `could not download Bazel ... not found`), so the
drill reproduces the exact post-bump observable state: pin lines in both
generated files moved 9.2.0 to 9.2.1 (3 lines), flag lines untouched.

- Skew: `preset.update_test` failed (`FAIL: preset pin`; `missing
  substring 1/8`), proving a bump without a reviewed regen goes red.
- Regen: `bazel run //tools/bazelrc:preset.update` restored the files; the
  flag diff under review was the 3 pin lines only, all 6 flag lines
  byte-identical: pin-only, no flag changes required.
- Disposition: the 9.2.1 bump was correctly not adopted (no such release;
  9.2.0 stays latest stable per dependency currency).
- Verification after regen: `preset.update_test` passes,
  `bazel build //...` succeeds, `bazel test //...` passes 40/40, corpus
  dogfood covers all 23 corpus targets with 46/46 evaluator passes at
  `--fail_on warning`, ownership audit reports zero uncovered.
- The branch was merged by hand; `renovate.json` sets no auto-merge.

The drill PR also closed a pre-existing red `main`: the
implementation-coverage gate had failed since the M04 completion-report
push (`quality/testdata/hinted.rs` declared by Bazel but missing from
`tools/coverage/inventory.txt`), and the WP3 sibling feature had left
three executable lines uncovered (`quality/markdown/src/lib.rs:323`,
the empty-resolution index branch; `quality/runner/src/real.rs:620,636`,
the initial/terminal diagnose error arms). The fix adds the `support`
inventory line for the fixture and three unit tests (self-directory
index resolution, initial-diagnose abort, poisoned-terminal abort);
the gate now reports `PASS 6453/6453 executable lines` with no errors.

## WP1 Corpus Ownership

22 `real_source_target(name = "corpus")` targets (one per package) own every
applicable file: `BUILD.bazel`, `MODULE.bazel`, `*.bzl`, `*.toml`, `*.md`,
excluding generated locks. The audit compares the tracked applicable set
against the Bazel-declared closure
`kind('source file', deps(kind(real_source_target, //...)))` with `comm`:
208 applicable files, zero uncovered. The reverse direction is informational
(9 files): `LICENSE`, the Vale config and style, `quality/result.proto`,
and fixture-owned files are corpus-referenced without being applicable
sources. Rust implementation sources stay dx-owned under the M02 wrappers;
the `rust/*` corpus targets own Starlark and manifests only.

Canonical policy is `real_fixture_policy` for corpus and fixtures alike.
Corpus-local configs are authoritative: Vale runs under
`//quality:corpus_vale_config` (with the narrow `Dx.Markers` style start)
and Clippy/Markdown sibling facts are bound per target through
`aspect_hints` / `markdown_siblings`.

## WP2 Adapters Over Corpus

Direct-tool runs are green corpus-wide: Buildifier `8.5.1 --mode=check`
clean on all 64 Starlark files, Taplo `0.10.0` clean on all 7 TOML files,
Vale `3.20.0` clean on all 132 Markdown files, and the repository-owned
`quality_markdown` checker clean on all 132 Markdown files (remotes
recorded-never-fetched).

Review correction recorded here: the WP2 Markdown direct run passed the
full 132-file workspace closure as `--source` inputs, which masked
undeclared cross-package links. The hermetic aspect path (each target sees
only its own sources plus declared siblings, undeclared fails closed)
rejected 4 lint results with `missing-file-target` findings for 22 link
targets across the root, `docs`, `.github`, and `libs/starlark` corpora.
The corpus declarations were completed accordingly (22 `markdown_siblings`
edges, e.g. root pages and the behavioral matrix for `//docs:corpus`;
12 docs pages for `//:corpus`): siblings resolve links only and are never
linted. After the fix all 44 aspect-produced results (22 lint + 22 format,
one per corpus target per capability) pass the per-result evaluator at
`--fail_on warning`.

## WP3 CI Enforcement And Measurements

CI (`.github/workflows/ci.yml`, job `corpus-dogfood`) enforces, on the seed
host class with local-only execution:

- Bazel-resolved tool execution: builds the pinned standalone tools and
  the evaluator; no quality tool is installed. Actions run with an empty
  `PATH` (unit-pinned), so ambient `rustfmt`/`clippy` copies on the runner
  are irrelevant.
- Corpus ownership audit: the `comm` gate above fails on any applicable
  file without a corpus owner.
- Direct checks: the 22 corpus targets built with `real_lint_aspect` and
  `real_format_aspect` over `--output_groups=dx_results`. Targets travel
  to `build`/`aquery` as literals from the recorded query: a bare query
  expression over `//...` widens the analyzed set under `aquery` (140
  nodes, reaching the manual negative fixture and aborting analysis), so
  the literal expansion is the supported invocation. Fixtures are excluded
  by construction: only `name = "corpus"` targets are selected, keeping
  the designed hinted-Clippy failure out of the gate.
- Evaluator thresholds: every produced result must pass
  `quality_evaluator --fail_on warning`, and every corpus target must own
  at least one result. Result paths to `bazel run` are absolute
  (`bazel run` does not execute from the workspace root).

The local dogfood commands (same invocations) are documented in
[Local Workflows](../contributing/local-workflows.md).

Action/cache behavior on the seed host (`bazel test //...`, 39 tests):

- No-op: `Executed 0 out of 39`, 0.23 s.
- Narrow Starlark change (comment-only in `libs/starlark/defs.bzl`):
  `Executed 1 out of 39`, 39 pass, 0.66 s.
- Narrow Rust change (comment-only in the Markdown checker):
  `Executed 1 out of 39`, 39 pass, 0.95 s.
- Config change (`--test_env` addition): `Executed 39 out of 39`,
  39 pass, 1.33 s.
- Warm `bazel build //...`: 235 action cache hits, 0.39 s.
- Corpus aspect build at steady state: fully cached (1 internal process).

Probe edits were reverted; the tree shows no probe residue.

## Commands And Results

- `bazel build //...`: success (warm: 235 action cache hits).
- `bazel test //...`: 39/39 pass across no-op, narrow-change, and
  config-change runs as tabulated above.
- Corpus aspect build (22 targets, both aspects): success, 44
  `DxRealQualityLint`/`DxRealQualityFormat` actions, one lint plus one
  format result per corpus target.
- Evaluator over all 44 corpus results at `--fail_on warning`: 44 pass.
- Ownership audit: 208 applicable files, zero uncovered.
- Buildifier `--mode=check --lint=warn` clean on all touched Starlark
  files; Vale clean on touched Markdown (stdin probe plus aspect stages).

## Changed Components And Public Labels

Corpus declarations in the root, `docs`, `.github`, and `libs/starlark`
`BUILD.bazel` files (sibling closures only); no rule, adapter, runner, or
policy code changed. CI gains the `corpus-dogfood` job; no ambient tool
dependency is added. No consumer-facing label or support claim is added.

## Documentation Changes And Unresolved Items

[Local Workflows](../contributing/local-workflows.md) now documents the
real build/test/coverage, corpus dogfood, and preset update-loop commands
(the design-only text is replaced). Open items: the `aquery`
query-expression widening noted above (worked around with literal
expansion, root cause not pursued); Renovate activation is configured but
not yet observed proposing (no newer stable Bazel exists today);
non-Linux platforms and the remaining O20 evidence stay gaps per
the M04 report.
