# M13 Completion Report: Repository-Corpus Quality Closure

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed and
no adapters were introduced: M13 closes ownership, generation, and quality
gaps with scoped configuration only.

## WP1: Ownership Inventory And Justified Exclusions

Applicable-class audit (same shape as the CI corpus-dogfood step):
`git ls-files` filtered to `BUILD.bazel`/`MODULE.bazel`/`*.bzl`/`*.toml`/`*.md`
minus `*.lock` gives 265 applicable files; `bazel query "kind('source file',
deps(kind(real_source_target, //...)))"` covers all 265 — zero uncovered, zero
duplicate `direct_sources` across the 36 `corpus` targets.

Reverse map: 419 tracked files, 142 outside the corpus closure, each Bazel-owned
or justified:

- First-party `*.rs`/`*.go` implementation sources: Bazel-owned as compilation
  inputs (`srcs`) of their wrapper/Go targets; corpus tracks declarations and
  configs, never implementation (M02/M05 policy).
- `gazelle/rust/testdata/**` golden tree: Bazel-owned via the
  `generation_testdata` filegroup (`glob(["testdata/**"])`) consumed as test
  data; excluded from generation by root `# gazelle:exclude
  gazelle/rust/testdata` (dual-clippy-config fixture would panic the updater).
- `quality/testdata/**` adapter fixtures: hand-maintained behind the
  hand-written `quality/testdata/BUILD.bazel`, consumed as test data by the
  corpus-owned test definitions in the same package; excluded from generation
  by root `# gazelle:exclude quality/testdata`.
- Shell runners (`dx/env/bootstrap_test.sh`, `env/doctor.sh`, `env/tool.sh`,
  `gazelle/rust/*_test.sh`, `rust/ide/ide_acquisition_test.sh`): Bazel-owned as
  `sh_test` `srcs`, never linted corpus sources.
- Proto schemas (`generation/result.proto`, `env/marker.proto`):
  Bazel-owned as `proto_library` `srcs`. `real_source_target` has no proto
  class (rust/starlark/toml/markdown only); `quality/result.proto` is
  corpus-reachable solely as a `markdown_siblings` input of `//docs:corpus`
  for the protocol doc, so no new corpus class was invented for compilation
  inputs with no doc consumer.
- Workspace/tooling config without a target class: `.github/workflows/ci.yml`,
  `.gitignore`, `.bazelrc`, `.bazelversion`, `MODULE.bazel.lock`,
  `renovate.json`, `rust/hello/Cargo.lock` (derived), `tools/coverage/inventory.txt`
  (derived), `tools/bazelrc/preset.py` (generator, no quality class) and
  `preset.bazelrc` (generated fragment) — the last two justified in-tree at
  `tools/bazelrc/BUILD.bazel`.
- Test-data text (`libs/starlark/tests/*_fixture.txt`,
  `quality/artifacts/update.py`, fixture `BUILD.in`/`BUILD.out`/`WORKSPACE`/
  `asset.txt` files): consumed as test data by corpus-owned test definitions.

## WP2: Direct-Bazel Supersedes Manual Checks

Generation freshness moved from manual diff inspection to the canonical public
path: `bazel run //dx:generate_check` (upstream `-mode diff` wiring from M10)
is clean (`EXIT=0`) and now runs in CI before the quality commands converge
anything (`.github/workflows/ci.yml`: "Verify generated-file freshness").
The corpus ownership audit above replicates the CI audit locally. The
`tools/coverage:coverage_bin` dep moved from a hand-kept edge the scanner
could not see (bare `coverage_gate::run` path) to an explicit `use
coverage_gate;` import pinned by a package-scoped resolve — the generator now
derives it instead of stripping it.

## WP3: Deterministic Generation, Quality, And Invalidation

Two generation diffs closed, both import-spelling mismatches, no language
changes. The Rust scanner resolves imports from `use`/`extern crate` items
only (`importsFor` in `gazelle/rust/lang.go`), and child `# gazelle:resolve`
directives win over root ones (proven with a scratch probe, then removed):

- `generation/result`: `lib.rs` spells the schema both as `use proto::{...}`
  (local alias) and `pub use result_proto::... as proto`; root maps both
  spellings at quality. Package-scoped resolves pin both to
  `//generation:result_proto_rs` (the `aquery` extern evidence shows quality's
  label would collide the `result_proto` extern), plus a load-group blank-line
  tamp-down.
- `tools/coverage`: explicit `use coverage_gate;` in `src/main.rs` plus a
  package-scoped resolve pinning it to `:coverage_gate` (plain `rust_library`
  providers are not indexed); the dep canonicalizes to
  `//tools/coverage:coverage_gate`. Both binaries run (`--help` usage exits
  normally).

Docstring-after-load `buildifier/no-effect` warnings fixed in 12 BUILD files
by moving the docstring first; `dx/cli` test/binary deps accept the
generator-derived `serde` + `//generation/result:generation_result` edges;
`# keep` annotations cover the remaining hand-authored rules
(`dx_env_bin_test`, `hello_derive`, `hello_cdylib`, `hello_staticlib`).

Invalidation (narrow changes rebuild only the expected): editing
`tools/coverage/src/main.rs` then `bazel build //tools/coverage/...` executed
2 sandbox actions against 30 cache hits; full `bazel build //...` afterwards
executed 0 processes; full `bazel test //...` executed 2 sandbox actions
against 715 cache hits.

## Evidence

- `bazel run //dx:generate_check`: `EXIT=0` (clean).
- `bazel run //dx/cli:dx -- lint --check`, `format --check`, `typecheck
  --check` over the 36 corpus targets: all `EXIT=0`, zero findings.
- `bazel build //...`: `EXIT=0`. `bazel test //...`: 82 pass, 0 fail.
- `bazel coverage //...`: 76 pass; `bazel run //tools/coverage:check -- ...`:
  `coverage gate: PASS 22511/22511 executable lines`.
- Bare `//...`-scoped `dx format`/`typecheck` still report the
  intentionally-dirty `quality/testdata` fixtures (and Cargo-only `rust/hello`
  externs) by design; the gates are corpus-scoped.

## Changed Components

`BUILD.bazel` (root excludes); `.github/workflows/ci.yml` (freshness step);
`docs/milestones/M13-completion-report.md` (new); 12 BUILD docstring fixes
(`dx/*`, `gazelle/rust`, `generation/result`, `quality/*`);
`generation/result/BUILD.bazel` (scoped resolves); `tools/coverage/BUILD.bazel`
(scoped resolve, canonical dep); `tools/coverage/src/main.rs` (explicit
`use`); `dx/cli`, `dx/env`, `rust/hello` BUILD dep/keep tamp-downs;
`docs/milestones/M12-completion-report.md` (rust.md link fix).

## Open Items

- Non-Linux hosts and remote execution are unproven (same gap class as M00).
- No proto corpus class exists; if a future milestone needs proto quality
  checks, `real_source_target` must grow the class rather than smuggling
  schemas in as docs siblings.
