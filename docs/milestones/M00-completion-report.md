# M00 Completion Report: Bazel, Rust, And CI Seed Quality

Seed host: Linux x86_64 glibc (`sdu-144133.pc.sdu.dk 7.0.0-30-generic`, Ubuntu 26.04.1 LTS).
CI host: `ubuntu-latest`, local execution, no remote. Non-seed required hosts from
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms) were
unavailable and are recorded as gaps, not claimed.

## Scope Completed And Deferred

All four work packages are complete: seed pins and toolchain (WP1), external-dependency
fixture (WP2), CI skeleton with gate enforcement (WP3), and the implementation-coverage
gate with inventory (WP4). No capability-term transitions are claimed; everything new is
bootstrap-maintained. No public APIs or load labels were added. Deferred per milestone
scope, not as gaps: formatters/linters and quality adapters (M04/M05), full host coverage
and reusable CI (M27).

## Exact Pins

Bazelisk v1.29.0 (Linux amd64 SHA
`5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992`) launching Bazel
9.2.0 via `.bazelversion`; `rules_rust 0.74.0`, `rules_cc 0.2.22`, Rust `1.98.0`
(`edition = "2021"`); `anyhow 1.0.104` (`Cargo.lock version=4`,
`MODULE.bazel.lock lockFileVersion: 28`). Format was checked with the toolchain's pinned
rustfmt nightly 2026-08-20 (`rules_rust` default), clean on all four sources.

## Commands And Results

Commits `f81e7a9` (WP1+WP2), `45c76b4` (WP4), `e1dc15e` (WP3). Each of these passes from
both the worktree and a clean clone (`/tmp/m00-clean-check`, fresh output base, 323
actions rebuilt): `bazel build //...`, `bazel test //...` (2/2: `hello_test` 4 tests,
`coverage_gate_test` 53 tests), `bazel run //rust/hello:hello`, `bazel coverage //...`
plus `bazel run //tools/coverage:check` over `bazel-out/_coverage/_coverage_report.dat`
reconciled against `tools/coverage/inventory.txt`: `PASS 1118/1118` (`hello` lib 27/27,
gate lib 1091/1091, both mains 0/0 ignored), exit 0. CI run `34383711555` on the WP3 push
completed success, including the `bazel coverage //...` and gate-enforcement steps.

## Coverage Evidence

The gate (`tools/coverage`) counts only LCOV `DA` lines, unions duplicate `SF` records by
max hits, and accepts textual Rust `//` ignores only with a nearby `reason:` (same or
previous line); support sources and non-Rust files never enter the denominator, and any
eligible-with-lines Starlark fails. Missing-report usage exits 1, never a pass; an empty
denominator is never a pass. Negative fixtures (all unit-tested, all failing closed):
missing report file, uncovered lines, stray `STOP`, reasonless markers, unknown
dispositions, uninventoried sources, and `.bzl` sources with lines. The two thin `main.rs`
shims carry reasoned `LCOV_EXCL_START/STOP` pairs; nothing else is ignored.

## Starlark Feasibility Evidence

M00 authors zero `.bzl` files (empty `**/*.bzl` glob; only loads of the pinned rulesets),
so there is no Starlark line coverage to claim and no behavioral fallback is used. The
technique stays feasible by construction: CI reconciles `bazel query 'kind("source file",
deps(//...))'` against the inventory, so any future authored `.bzl` surfaces as either
missing-from-inventory (usage error) or, if marked eligible with lines, a hard gate
failure pinned by the `starlark_line_data_fails_until_route_exists` unit test under the seed Bazel.

## Deviations And Blockers Hit

`bazel sync --only=crates` does not exist in Bazel 9, so the repin command is
`CARGO_BAZEL_REPIN=1 bazel build //rust/hello:hello` (MODULE.bazel corrected).
`coverage --instrumentation_filter` matches target labels (`package:target` joined by
`:`), so the `^//rust/,^//tools/coverage/` prefixes silently skipped the gate's own test
target and produced empty `coverage.dat`; the filter is now `^//` with enforcement scoped
by the inventory instead. Bazel 9 coverage requires
`coverage --test_env=GENERATE_LLVM_LCOV=1` on GCC hosts (rules_rust#3948); the gate must
be pointed at the absolute combined-report path (relative `bazel-out/...` resolves inside
the runfiles tree and fails closed with exit 1).

## Documentation Changes And Unresolved Items

Changed: this report, the M00 index link below, `.bazelrc`, `MODULE.bazel`,
`rust/hello` seed, `tools/coverage` gate plus inventory, and `.github/workflows/ci.yml`.
Open and untouched: O14 follow-ups outside seed scope, O46 inventory/WP prerequisites
(explicitly not bootstrap scope), and the unavailable-host gaps above. Milestone
exclusions (Starlark test APIs, wrappers, product adapters, CLI workflows) were respected.
