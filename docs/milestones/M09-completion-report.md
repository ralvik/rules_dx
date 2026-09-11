# M09 Completion Report: Rust-First Gazelle

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed.

WP1 (Rust-local indexing, exact mapping/ignore, naming, desired/empty-rule, and
result primitives), WP2 (conventional crate/module/binary/unit-test/integration-test
ownership), and WP3 (Cargo target/dependency authority and deterministic stale
cleanup for the initial shapes) are complete per the
[M09 milestone](M09-rust-first-gazelle.md). No silent deferrals. No
capability-term transitions are claimed: the extension is first-party
implementation under test, not a product support claim.

## WP1: Indexing, Mapping, Naming, Desired/Empty Rules

`gazelle/rust/lang.go` implements the private `rust` Gazelle language
(`NewLanguage`, `//gazelle/rust:rust`):

- Deterministic ASCII target naming (`naming.go`, `Normalize`,
  `UnitTestName`, `IntegrationTestName`) with collision rejection.
- Local rule indexing via Gazelle `RuleIndex`: only `dx_rust_library`
  crate names are indexed (`Imports`); binaries/tests resolve through the
  index but are never indexed themselves (single ownership).
- Exact Gazelle `# gazelle:resolve rust <imp> <label>` override precedence,
  then single-match index resolution with deterministic bytewise-sorted `deps`.
  Zero matches fail as unresolved (fail-closed); multiple matches fail as
  ambiguous with sorted candidates. Self-imports are dropped.
- Inherited exact `# gazelle:dx_ignore_import rust <imp>` with malformed,
  mapping-conflict, and stale-use rejection. Stale ignores fail in
  `AfterResolvingDeps`; mapping+ignore on the same import fails.
- Desired/empty rule metadata (`KindInfo` with `MatchAttrs: crate_root`,
  `NonEmptyAttrs: crate/srcs`, `MergeableAttrs`, `ResolveAttrs: deps`) and
  conservative stale cleanup: only owned `dx_rust_*` kinds not in the desired
  set are emptied; foreign kinds (e.g. `filegroup`) are never touched.
- Aggregated errors abort through `AfterResolvingDeps` panic before BUILD
  emission, so failed generations write no BUILD file (proven by
  `failure_test.sh`).

## WP2: Source-Only Ownership

`layout.go` + `parser.go` implement conventional source-only discovery:

- Recognized roots: `src/lib.rs`, `src/main.rs`, direct `tests/*.rs`.
  Fallback crate name is the normalized directory basename; lib+bin collision
  yields `<name>` (lib) and `<name>_bin` (thin bin). No `src/bin`,
  examples, benches, or build-script inference.
- Narrow parsing: `mod` (with `#[path]`), literal `use` paths,
  `extern crate`, `#[test]`, exact `#[cfg(test)]` (attribute, inner, and
  module forms). Comments, strings, and macro definitions are inert.
- Nested module ownership with orphan-module, dual-owner, and normalized-name
  collision failures. `tests/common.rs`-style direct roots stay independent
  integration tests; nested `tests/common/mod.rs` helpers are owned modules,
  never reclassified roots.
- Wrapper generation: `dx_rust_library` / `dx_rust_binary` plus one
  `dx_rust_test` per crate with unit tests (`<target>_test`, `crate: :<target>`,
  no `srcs` duplication) and one `dx_rust_test` per direct `tests/*.rs`
  root. Unit-test-only imports attach only to the unit-test target.
- Standard-library filtering (`std`/`core`/`alloc`/`crate`/`self`/`super`
  plus local modules).

## WP3: Cargo Authority And Stale Cleanup

`cargo.go` implements ordinary-target Cargo authority:

- Checked-in `Cargo.toml` owns target declarations: `[package].name`,
  `[lib]`/`[[bin]]`/`[[test]]` `name`/`path`/`harness`, plus implicit
  `src/lib.rs` / `src/main.rs` / `tests/*.rs` when not explicitly declared.
  Missing roots, duplicate names/paths, and empty normalized names fail.
- Production imports must be in `[dependencies]`; test imports may add
  `[dev-dependencies]`. Undeclared production/test imports fail with manifest
  context. Local (path) deps resolve through the Gazelle index; external deps
  emit exact `crate_deps([...], package_name = "<bazel pkg>")` +
  `aliases(normal=True[, normal_dev=True], package_name=...)` through the
  public `@crates//:crates.bzl` macros. `package_name` is the Bazel package
  path, never Cargo's package name. Private lockfiles (`Cargo.Bazel.lock`,
  `cargo-bazel.json`, crate_universe maps) are never read.
- `edition` is copied verbatim; `harness = false` maps to
  `use_libtest_harness = False`. `required-features`/`crate-type`/`proc-macro`
  fail with kept-target guidance (M12 scope).
- Deterministic stale cleanup and merge preservation proven by golden
  fixtures: `source_only` (lib+bin+integration), `cargo/app` (edition,
  external `crate_deps`/`aliases`), `merge/binary` (stale `old_bin` removal),
  `merge/cargo_trim` (stale test removal, Cargo shape), `merge/merge`
  (`# keep` comments, `data`/`tags`/`visibility`/`crate_name` preserved,
  `gone` removal), `merge/empty` (no-op). Unrelated `filegroup(assets)` and
  `retained` kept rules are preserved; no sidecar is created.

## Generated Shapes, Syntax, Strict-Resolution, Blocked Mappings

- Shapes: library (`demo`), thin binary (`demo_bin` on lib+bin collision),
  binary (`demo`), crate unit test (`<target>_test`), integration test
  (`<stem>_test` without duplicating `_test`).
- Recognized syntax/layouts: `src/lib.rs`, `src/main.rs`, `tests/*.rs`;
  `mod`/`#[path]`/literal `use`/`extern crate`/`#[test]`/exact `#[cfg(test)]`.
- Strict-resolution cases: override precedence, single-index hit, self-drop,
  unresolved fail-closed, ambiguous fail-closed with sorted candidates,
  mapping-vs-ignore conflict, inherited-ignore consumption, stale-ignore
  failure.
- Blocked Cargo mappings: `required-features`, `crate-type`/`proc-macro`,
  non-ordinary kinds, `dylib`/multi-kind, examples/benches/build scripts
  (all M12/O24). Manifest errors carry `Cargo.toml` path, line, and field.

## Public Surface

No public `dx generate` was added. `//gazelle/rust:rust` (`go_library`) and
`//gazelle/rust:gazelle` (`gazelle_binary`) remain `//visibility:private`.
Generated rules load existing public wrappers from
`@rules_dx//rust/rules:defs.bzl` and externals from
`@crates//:crates.bzl` (`aliases`, `crate_deps`). No new load label is public.

## Evidence

Exact commands on this host (2026-09-11):

- `bazel build //...`: success, 189 targets.
- `bazel test //...`: 62/62 pass, including `//gazelle/rust:rust_test`
  (Go unit/API-boundary tests), `//gazelle/rust:generation_test` (golden
  `source_only`/`cargo`/`merge` fixtures), `//gazelle/rust:failure_test`
  (unresolved fails with no BUILD write; inherited ignore permits; stale
  ignore fails), `//gazelle/rust:idempotent_test` (two consecutive runs
  byte-identical).
- `bazel coverage //... --combined_report=lcov --test_tag_filters=-no-coverage`
  plus `bazel run //tools/coverage:check -- --report
  $PWD/bazel-out/_coverage/_coverage_report.dat --inventory
  $PWD/tools/coverage/inventory.txt --sources /tmp/implementation_sources.txt
  --root $PWD` (sources from `bazel query 'kind("source file", deps(//...))'`
  filtered to `*.rs`/`*.go` per CI): **PASS 17580/17580** executable lines.
  Go: `cargo.go 181/181`, `lang.go 479/479 (4 ignored)`,
  `layout.go 152/152`, `naming.go 67/67`, `parser.go 628/628`.
  Rust gate from M08 retained (e.g. `exec.rs 2611/2611 (55 ignored)`,
  `reports.rs 1403/1403 (5 ignored)`, `resolve.rs 1130/1130 (16 ignored)`).
  `doc.go` (0 lines) and all `*_test.go` plus 11 Rust fixture sources are
  `support`, never in the denominator. The two process-spawning sh_tests and
  `generation_test` carry `no-coverage` (instrumenting the child changes the
  asserted stderr contract); they run under `bazel test //...`.
- `bazel test //dx/cli:dx_cli_fmt_test //dx/cli:dx_cli_clippy_test`:
  pass; rustfmt via the Bazel toolchain config, Clippy with warnings as errors.
- Corpus audit (`git ls-files` applicable vs `deps(kind(real_source_target))`
  closure): zero unexplained exclusions. Idempotent generation creates no
  integration sidecars or operational activation.

Coverage inventory reconciles against Bazel-declared `*.rs`/`*.go` sources;
`LCOV_EXCL_*` markers with nearby `reason:` are validated in the gate. No
Starlark was authored in M09, so no Starlark fallback applies. No benchmarks
beyond deterministic-ordering/idempotence fixtures are claimed.

## Changed Components

New private `gazelle/rust/` (`cargo.go`, `lang.go`, `layout.go`, `naming.go`,
`parser.go`, `doc.go`, `*_test.go`, `failure_test.sh`, `idempotent_test.sh`,
`testdata/{source_only,cargo,merge}/...`, `BUILD.bazel` with `rust`,
`gazelle`, `generation_test`, `failure_test`, `idempotent_test`,
`rust_test`, `corpus`). Modified `MODULE.bazel` (+`rules_go 0.63.0`,
`gazelle 0.52.2`, `rules_shell 0.6.1`, `go_sdk 1.26.6`) and lockfile; docs
`generation/rust.md` (ordinary-target mapping), `open-decisions.md` (O22
resolved for initial mapping), `milestones/README.md`; coverage
`tools/coverage/{inventory.txt,src/lib.rs}` (Go measured, `.go` in scope),
`tools/bazelrc/{preset.py,preset.bazelrc,bazelrc-preset.bzl,preset_tests.bzl}`
and `.github/workflows/ci.yml` (coverage preset: `-no-coverage` filter,
`COVERAGE_GCOV_PATH=/usr/bin/gcov` for rules_go's C/C++ helper; CI sources
now `*.(rs|go)`).

## Deviations, Gaps, Exclusions

No deviations from the milestone scope. Out of scope respected: shared
cross-language abstractions, public `dx generate`, native config binding,
advanced Cargo targets/build scripts/examples/benchmarks, and other languages
untouched. Deferred: exact accepted-ignore notices (require M10 result
transport; no sidecar introduced). Gaps: non-Linux platforms, remote/cache
qualification, and external-consumer evidence remain for M27+; advanced Cargo
remains M12/O24.
