# Verification matrix (language x layer)

Close-out status page:
which verification layer covers which language, with as-built evidence only.
No cell here is a `Supported` claim; promotion to `Supported` requires
release evidence per the [support matrix](../product/support-matrix.md).
Open cells are recorded as gaps; docs describe as-built behavior only.
No cell is `Supported`: missing test/release evidence blocks promotion until
platform plus consumer plus release evidence passes per the support matrix,
enforced by `bazel run //tools/ci:supported_evidence_gate`.
Non-dogfed execution plan delivered (see issue #508 for remaining execution gaps;
hermetic CLI-contract pins under issue #407)
(`bazel run //tools/ci:non_dogfed_paths`): CLI-contract coverage runs
hermetically under `bazel test //...` (no nested Bazel), negative fixtures
via explicit failure proofs, the no-coverage cohort via coverage-excluded
runs, and shell sources via ownership plus test execution with no quality
class by design — never silently under the standard dogfood gates.

## Layers

- **Corpus dogfood**: `real_source_target(name = "corpus_*")` per content type
  per package (issue #15, shared `tags = ["corpus"]`)
  checked with the real lint/format aspects at `--fail_on warning`, plus
  the corpus and code ownership audits. Scope: Markdown/Starlark/TOML
  target-less files (corpus) and production code on normal targets
  (lane A).
- **Layer-2 runner matrix**: real `quality_runner` over provider-less
  data, one case per supported language x capability cell x {pass, fail}
  ([quality/runner-matrix](../quality/runner-matrix.md)).
- **Generation freshness**: `dx generate --check //...` must be a
  deterministic no-op on a clean checkout; BUILD/corpus sync is owned by
  generation (open work under issue #503).
- **Examples external-consumer**: per-foundation `adopt-*` workspaces
  proving generation as a consumer, plus acquisition/laziness proof
  (delivered on the seed host; platform/remote dimensions owned by
  #298/#507).
- **Layer-4 CLI-contract (issue #407, hermetic, replaces nested E2E)**:
  `dx test`/`dx build` exit-code preservation via `cli/cli/src/exec` unit
  pins (including Bazel test-failure code 3), format rewrite `x=1` →
  `x = 1` via `//quality/testdata:runner_matrix` goldens, aspect wiring via
  `DxSubjectInfo` pins (`//quality/testdata:real_aspect_presence`), preset
  check→update→recheck via `//:preset_parity_test`; `local_path_override` +
  `dx_dev` wiring via the adopt-rust smoke in normal CI
  (`bazel build //examples/adopt-rust/... --config=dx_dev` in the build job).
  What is lost: real-daemon exit 3, real Buildifier rewrite, full consumer
  wiring now smoke-only.
- **Dependency checks**: required-core plus admitted lockfile-consistency and
  declared-dependency usage fixtures delivered in `tools/depcheck/`
  (issue #22; remaining opens under issue #510); framework-composition depcheck stays with the JS/TS pnpm route.
- **Audit/update live execution**: `dx update` resolver backends per set with independent-set
  continuation and per-set reporting delivered (issue #19); `dx audit` auditor wiring, advisory
  acquisition with 24h cache semantics and offline matching, plus SARIF/SPDX mapping delivered
  (issue #18). `Audit/update` here records repo-wide ecosystem live execution, distinct from
  per-language source audit in the [support matrix](../product/support-matrix.md#application-foundations).
- **Docs pipeline**: per-language adapter runs, link/reference proofs,
  renderer/site artifacts, cache and determinism measurements, guide prose with
  guide-step verification, and first-hour timing proof stay open under issue #421
  (live successor to closed #310)
  (see [Documentation](../documentation/README.md#contracts)); no working site claimed.
  IR plus planning records with fixture evidence qualified seed-only under #421
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR schema,
  codec roundtrip/parity/ordering/compat, dx_docs planning units, frozen
  contracts, removed stub behind ADR 0020, with adapter runs, renderer/site
  execution, rebuild proof, link completeness, guide-step wiring, timing proof,
  and pin-bump/drift as owned gaps).
- **Environment/codegen**: deferred/unsupported records plus fixture
  evidence qualified seed-only under #506
  (`bazel run //tools/ci:env_codegen_qualification`; public protocol,
  Windows fallback, standalone, signing/trust, plus bootstrap/lock/roots/
  collector/env-plan/node projection evidence with unproven tests as owned
  gaps).
- **Non-dogfed execution plan**: the cohorts that never run under the
  standard dogfood gates each have an explicit path, pinned by
  `bazel run //tools/ci:non_dogfed_paths` (issue #508; hermetic pins under issue #407): CLI-contract
  via hermetic pins under `bazel test //...` (no nested Bazel, no manual;
  plus the adopt-rust `dx_dev` smoke in normal CI; loss recorded here);
  negative fixtures via green hermetic proofs (issue #406: sh_test goldens +
  failure_test, no manual, CI `test` job via `bazel test //...`); the no-coverage cohort via coverage-excluded runs (preset
  `test_tag_filters=-no-coverage`, `target_tags` skip proof,
  `coverage_cell`/`coverage_qualification` gates, CI `test` execution);
  shell sources with no quality class by design (`shell` known but no
  `shell_srcs` owner, corpus/code-ownership filters exclude `.sh`, every
  `.sh` in `deps(//...)`, execution via `bazel test //...`, portability via
  `shell_contract`).

## Status

`Delivered` means implemented and verified on the Linux x86_64 seed host
plus Linux arm64 native (issue #410) plus the two Linux static-musl
profiles (issue #411) plus macOS arm64 native (issue #412) plus macOS
x86_64 best-effort native (issue #413) plus Windows x86_64 MSVC-compatible
native (issue #414). `Open` means open work under issues #506-#512 with no implementation
claimed here. `Planning only` means planning is implemented with live
execution deferred. No report-only status remains per ADR 0022 (no standing benchmarking).

| Language | Corpus dogfood | Layer-2 matrix | Generation | Examples | E2E | Depcheck | Audit/update | Docs | Env/codegen |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Delivered | Delivered | Delivered | Delivered (`adopt-rust`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Python | Delivered | Delivered | Delivered | Delivered (`adopt-python`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| JavaScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| TypeScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Go | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-go`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Java | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-java`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Kotlin | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-kotlin`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Scala | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-scala`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| C# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-csharp`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| F# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-fsharp`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| C++ | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-cpp`) | Delivered (contract) | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Vue/Svelte/Astro/MDX | Delivered (code ownership) | Open (regions) | Delivered | Open (composition) | Delivered (contract) | Open | Delivered | Open | Open |

`Audit/update Delivered` above is ecosystem live execution delivered repo-wide; it stays
consistent with per-language source-`Audit` scope in the support matrix (`Not planned` for
Rust, JavaScript, TypeScript, Vue, Svelte, Astro, MDX; open tooling work for Python).
`Support-matrix Planned` cells claim accepted scope only; where this matrix shows `Open`,
the corresponding `Planned` cell is scope with open implementation.

Framework composition evidence (exact parser/compiler, provider,
generated-region, dependency, test, environment/IDE, quality-region
mappings) is pinned by
`bazel run //tools/ci:foundation_maps`, with owning qualification in
[Framework adapters](../generation/framework-adapters.md#framework-mapping-qualification). Language
provider/import/lock/tool-graph proofs are pinned by
`bazel run //tools/ci:foundation_maps`, with owning qualification in
[Generation](../generation/README.md#language-mapping-qualification),
[Environments](../environments/README.md#language-mapping-qualification), and
[Tools](../tools/README.md#language-mapping-qualification). Quality
family taxonomy stays open under
open work under issue #512.

## Battery

Accepted record of the as-built close-out battery. The full battery runs in
`.github/workflows/ci.yml` on a clean tree on the Linux x86_64 seed host
plus Linux arm64 native (issue #410, `ubuntu-24.04-arm` runners with a
separate `bazel-arm64-` disk-cache scope) plus the two static-musl profiles
(issue #411, Linux runners with per-profile `bazel-musl-*` scopes) plus
macOS arm64 native (issue #412, `macos-14` runners with a separate
`bazel-macos-arm64-` disk-cache scope) plus macOS x86_64 best-effort native
(issue #413, `macos-15-intel` runners with a separate
`bazel-macos-x86_64-` disk-cache scope; `macos-13` retired December 2025,
`macos-15-intel` until August 2027; gaps never block required-host release)
plus Windows x86_64 MSVC-compatible native (issue #414, `windows-latest`
runners with shell `bash` and a separate `bazel-windows-x86_64-` disk-cache
scope). The host matrix across these hosts is pinned by `bazel run
//tools/ci:ci_matrix_qualification` (issue #415, after portable-shell
#323):

Close-out battery plus docs gate qualified seed-only under #467
(`bazel run //tools/ci:closeout_battery_qualification`; battery commands
plus docs-ci gate pinned with CI wiring in `dogfood-freshness`; platform
plus consumer plus release evidence stays owned gap; no Supported claim).

- `build`: `bazel build //...` plus the adopt-rust `dx_dev` smoke
  (`bazel build //examples/adopt-rust/... --config=dx_dev`) for
  `local_path_override` + `dx_dev` wiring (issue #407, normal CI, no nested Bazel).
- `build-arm64`, `test-arm64`, `coverage-arm64`: the same build plus
  `dx_dev` smoke, `bazel test //...`, and the arm64 per-cell coverage gate
  (`dx coverage --min-coverage 97 //...` against
  `tools/coverage/arm64-inventory.txt`, summary only, no cross-cell union)
  natively on Linux arm64 (issue #410).
- `build-musl-x86_64`, `build-musl-arm64`: Rust musl std proof (`bazel query
  @rust_toolchains//... | grep musl`) plus `bazel build //...`
  (exec-platform tools) for the static-musl closures (issue #411; dynamic
  musl explicitly out of scope).
- `coverage-musl-x86_64`, `coverage-musl-arm64`: the musl per-cell coverage
  gates (`dx coverage --min-coverage 97 //...` against
  `tools/coverage/musl-*-inventory.txt`, summary only, no cross-cell union)
  on their Linux runners (issue #411).
- `build-macos-arm64`, `test-macos-arm64`, `coverage-macos-arm64`: the same
  build plus `dx_dev` smoke, `bazel test //...`, and the macos arm64
  per-cell coverage gate (`dx coverage --min-coverage 97 //...` against
  `tools/coverage/macos-arm64-inventory.txt`, summary only, no cross-cell
  union) natively on macOS arm64 (`macos-14`, issue #412; host-installed
  SDK fallback never approved, no secrets, no interactive acceptance).
- `build-macos-x86_64`, `test-macos-x86_64`, `coverage-macos-x86_64`: the same
  build plus `dx_dev` smoke, `bazel test //...`, and the macos x86_64
  best-effort per-cell coverage gate (`dx coverage --min-coverage 97 //...`
  against `tools/coverage/macos-x86_64-inventory.txt`, summary only, no
  cross-cell union) natively on macOS x86_64 best-effort (`macos-15-intel`,
  issue #413; host-installed SDK fallback never approved, no secrets, no
  interactive acceptance; gaps never block required-host release).
- `build-windows-x86_64`, `test-windows-x86_64`, `coverage-windows-x86_64`:
  the same build plus `dx_dev` smoke, `bazel test //...`, and the windows
  x86_64 per-cell coverage gate (`dx coverage --min-coverage 97 //...`
  against `tools/coverage/windows-x86_64-inventory.txt`, summary only, no
  cross-cell union) natively on Windows x86_64 MSVC-compatible
  (`windows-latest` with shell `bash`, issue #414; toolchains_msvc
  clang-cl/Microsoft-STL backend provisional with immutable lazy fetch,
  explicit EULA acceptance never automatic, installed Build Tools fallback
  never approved, no secrets, usage vs redistribution reviewed separately).
- `test`: `bazel test //...` (no manual tests; hermetic CLI-contract pins run here).
- `coverage`: `bazel run //cli/cli:dx -- coverage --min-coverage 97 //...`
  (seed cell only) plus `bazel run //tools/ci:coverage_report_guards`.
- `prove`: `:target_tags`, `:coverage_cell`,
  `:coverage_spill`, `:coverage_qualification`, `:musl_qualification`,
  `:macos_qualification` (arm64 plus x86_64 best-effort),
  `:windows_qualification`,
  `:ci_matrix_qualification` (host matrix runners plus caches plus refusal
  plus sharding plus portable shell, issue #415), `:release_hygiene`,
  `:release_policy`, `:publish_trust`, `:shell_contract`.
- `dogfood-freshness`: `bazel run //cli/cli:dx -- generate --check //...`,
  `//tools/ci:corpus_audit`, `:code_ownership`, `:non_dogfed_paths`,
  examples READMEs and
  laziness proofs, quality-cache aquery, depcheck contract, audit/update
  guards, wrapper sources, foundation maps, registry singularity, backlog
  contracts, GHCR hygiene and publish guards,
  widen-update loop, quality/distribution/backlog guards,
  `:supported_evidence_gate`, `:quality_adapters_parity`,
  `:env_codegen_qualification`, `:docs_pipeline_qualification`,
  `:consumer_ci_qualification`, `:file_family_qualification`,
  `:helper_qualification`, `:clap_tokenizer_qualification`,
  `:hello_smoke_qualification`, `:parser_sample_qualification`,
  `:rustfmt_edition_qualification`, `:cc_optout_qualification`,
  `:shell_env_qualification`, `:bindgen_qualification`,
  `:cxx_identity_qualification`, `:exact_target_qualification`, `:junit_qualification`,
  `:xunit_qualification` (xUnit v3 4.0.0 pins plus fixture evidence,
  issue #477), `:gotest_qualification`, `:googletest_qualification` (GoogleTest
  v1.18.0 pins plus C++17 floor plus fixture evidence, issue #479),
  `:scalatest_qualification` (ScalaTest 3.2.20 pins plus fixture evidence,
  issue #480), `:paket_qualification` (Paket files plus sha512 pins plus
  fixture evidence, issue #482), `:godeps_qualification` (Go `go.mod`/`go.sum`
  via `from_file` pins plus fixture evidence, issue #483), `:cc_hermetic_qualification` (C/C++
  sha256-integrity plus no-system-package pins plus fixture evidence, issue
  #484),   `:jvm_quality_qualification` (JVM versions plus rule-sets pins plus
  fixture evidence, issue #485), `:scala_dotnet_defaults_qualification`
  (Scala + .NET versions plus rule-sets pins plus fixture evidence, issue
  #486), `:native_quality_qualification` (native versions plus
  rule-sets pins plus fixture evidence, issue #487),
  `:structured_defaults_qualification` (structured versions plus
  rule-sets pins plus fixture evidence, issue #488),
  `:musl_qualification`, `:macos_qualification` (arm64 plus x86_64
  best-effort), `:windows_qualification`, `:ci_matrix_qualification`
  (host matrix, issue #415), and `:closeout_battery_qualification`
  (battery commands plus docs gate, issue #467).
- `devcontainer-check`, `docs-ci`, `consumer-ci` (all-enabled self-call on
  linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus
  windows_x86_64, issue
  #408, verbatim `//...`).

Green here (static guards on a clean tree, no full rebuild):
`non_dogfed_paths`, `supported_evidence_gate`, `distribution_closeout_guards`,
`env_codegen_qualification` 23/23, `docs_pipeline_qualification` 33/33,
`consumer_ci_qualification` 30/30, `file_family_qualification` 24/24,
`helper_qualification` 27/27, `clap_tokenizer_qualification` 19/19,
`hello_smoke_qualification` 16/16, `parser_sample_qualification` 23/23, `rustfmt_edition_qualification` 20/20, `cc_optout_qualification` 18/18, `shell_env_qualification` 15/15, `bindgen_qualification` 16/16, `cxx_identity_qualification` 18/18, `exact_target_qualification` 16/16, `junit_qualification` 16/16, `xunit_qualification` 16/16, `gotest_qualification` 16/16, `googletest_qualification` 16/16, `scalatest_qualification` 16/16, `paket_qualification` 16/16, `godeps_qualification` 16/16, `cc_hermetic_qualification` 16/16, `jvm_quality_qualification` 16/16, `scala_dotnet_defaults_qualification` 17/17, `native_quality_qualification` 17/17, `structured_defaults_qualification` 17/17, `musl_qualification` 12/12, `macos_qualification` 12/12,
`windows_qualification` 13/13, `ci_matrix_qualification` 14/14, `closeout_battery_qualification` 24/24.
Full `build`/`test` green is owned by CI on this tree via `bazel run //tools/ci:closeout_battery_qualification` (issue #467;
battery commands plus docs gate pinned, full rebuild owned by CI jobs, not re-claimed here).

Remaining reds stay owned gaps, not green claims:

- Full-tree `dx lint/format/typecheck/test --check //...` over fixtures and
  testdata stays open under #12 (lane A only) and #325 (consumer honesty).
- Per-cell coverage is qualified for the seed plus arm64 plus two static-musl plus macos arm64 plus macos x86_64 best-effort plus windows x86_64 cells under
  #507/#410/#411/#412/#413/#414 (`tools/coverage/cells.txt`,
  `bazel run //tools/ci:coverage_qualification`; no union, Starlark fallback,
  Codecov opt-in, quotas, local-only remote evidence). First-party PR reporting is
  adopted under #254 (Codecov opt-in only; the seed cell owns the PR comment,
  the arm64 plus musl plus macos plus macos-x86_64 plus windows cells report to their job summaries; the macos x86_64 best-effort cell reports without blocking required-host release). All required plus best-effort cells are qualified; out-of-v1 hosts stay platform-gated under #298.
- Docs pipeline and environment/codegen stay open under #421 and #506 (see
  [Documentation](../documentation/README.md#contracts)). Environment/codegen
  deferred records plus fixture evidence are qualified seed-only under #506
  (`bazel run //tools/ci:env_codegen_qualification`; no junction/copy
  fallback, no checksum-only fallback, no third-party plugin claim; bootstrap,
  fidelity, spaces, stale-clean, IDE, atomic-commit, BEP, projection,
  root-candidate tests stay owned gaps). Docs-pipeline IR plus
  planning records with fixture evidence are qualified seed-only under #421
  (live successor to closed #310)
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR, codec,
  planning, frozen contracts, removed stub; adapter runs, renderer/site
  execution, rebuild proof, link completeness, guide-step wiring, timing proof,
  and pin-bump/drift stay owned gaps; no working site claimed).
- Consumer-CI contract plus caller plus gate/aggregate fixture evidence
  qualified seed-only under #509
  (`bazel run //tools/ci:consumer_ci_qualification`; nine checks, explicit
  platforms, fail-closed sequential, stable dx-ci aggregate, hygiene,
  concurrency, permissions, per-cell coverage with fork-safe comments,
  all-enabled self-call (issue #408, verbatim `//...`), native bump loop
  (sole updater, issue #461),
   dx migrate syntax plus manifest selection (delivered CLI with fail-closed
   execution, issue #462) plus dx run multirun (issue #463 delivered), tag
   hygiene as-built; platform, runner, isolation, cache, ordering, merge,
   diff, queue, cancellation, aggregate binding, thread identity, ordering,
   limits, fork, untrusted, sensitive, retries, Code-Scanning, sequential,
   tag/release, native-bot, migrate-manifest gaps stay owned gaps);
   platform qualification
   beyond the seed plus arm64 plus musl plus macos plus macos-x86_64 plus
   windows hosts stays open under #298 (arm64 qualified under #410, static
   musl under #411, macos arm64 under #412, macos x86_64 best-effort under
   #413, windows x86_64 under #414).
- File-family quality record with fixture evidence qualified seed-only under #489
  (`bazel run //tools/ci:file_family_qualification`; provider-class
  applicability with never-suffix inference, Starlark/Buildifier plus TOML/Taplo
  adapter-backed evidence, parity-deferred CSS/djlint/buf/yaml/keep-sorted/shell
  plus cue/jsonnet/pkl/qml/terraform routes with owner plus frozen acquisition;
  deferred adapter execution, exact pins/digests/rule-sets/mappings, and platform
  plus consumer plus release evidence stay owned gaps; no Supported claim).
- Hand-rolled helper decisions with fixture evidence qualified seed-only under #315/#395
  (`bazel run //tools/ci:helper_qualification`; adopted digest via
  hex/blake3/sha2, diff via similar, SPDX parse via spdx, date calendar via
  chrono, scratch via tempfile, dir sizing and walks via walkdir/ignore/globset,
  LCOV `SF`/`DA` parsing via lcov with stays-hand-rolled atomic lock, path ladder,
  LCOV ignore scanner reasons; upstream re-evaluation plus any future migration stays owned gap).
- Date engine stays `chrono` with fixture evidence qualified seed-only under #398
  (`bazel run //tools/ci:helper_qualification`; `jiff 0.2` spike rejected:
  trivial day-granularity gates need no `tzdb`, heavier bundle/tree plus
  mechanical churn for a pre-`1.0` single-owner crate; re-evaluate on `jiff 1.0`).
- Clap-as-tokenizer legacy errors with fixture evidence qualified seed-only under #316
  (`bazel run //tools/ci:clap_tokenizer_qualification`; frozen legacy output as
  contract with snapshots across env, codegen shard, env shard, evaluator,
  runner, markdown, plus dx CLI tokenizer via disable_help_flag plus
  allow_hyphen_values plus invalid_token/parse_error mapping for unknown,
  missing, malformed, hyphen-value, and attached-echo shapes; strict clap
  parsing with auto help stays owned gap).
- Hello smoke as test with fixture evidence qualified seed-only under #464
  (`bazel run //tools/ci:hello_smoke_qualification`; 11 binary hellos with
  `hello_output_test` under `bazel test //...`, no-coverage Linux-only runfiles
  wiring with `dx_realpath`, component-only fixtures with no binary by design;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- Parser samples with fixture evidence qualified seed-only under #465
  (`bazel run //tools/ci:parser_sample_qualification`; every adapter-backed
  tool keeps a parser with pass plus fail samples — biome lint plus format,
  buildifier, clippy plus rustc via the shared rust diagnostics, eslint,
  flake8, markdown_check, prettier, pydoclint, pylint, ruff lint plus format,
  rustfmt, taplo lint plus format, tsc, ty, vale — with recorded Clippy/rustc
  diagnostics byte-identical to the Layer-2 matrix and tsc pipeline-only by
  design; platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Rustfmt crate edition with fixture evidence qualified seed-only under #468
  (`bazel run //tools/ci:rustfmt_edition_qualification`; the crate `CrateInfo`
  edition reaches `rustfmt --edition` with `RUST_EDITION` fallback and
  fail-closed missing edition on check plus fix, the 2015 `edition_2015_lib`
  with `edition_fmt_test` under `bazel test //...`, the Layer-2
  `matrix_rust_format_edition_2015` pass plus
  `matrix_rust_format_edition_mismatch` wrong-edition syntax-error cells over
  the real toolchain rustfmt, and live aspect `--tool-edition` pins;
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- CC opt-out linker with fixture evidence qualified seed-only under #471
  (`bazel run //tools/ci:cc_optout_qualification`; kept
  `use_cc_toolchain = 0` executes for pure-Rust scripts with sysroot
  `rust-lld` fallback plus `no_cc` stubs and empty flags, the
  `rust/tests/fixtures/cc_optout/` script plus lib plus `cc_optout_test`
  under `bazel test //...`, live execution-action markers, and `dx generate
  --check` stability; scripts needing CC still fail clearly when opted out;
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Shell-env default vs annotation extension with fixture evidence qualified seed-only under #472
  (`bazel run //tools/ci:shell_env_qualification`; first-party
  `use_default_shell_env = 0` emission plus `scripted` golden, global
  `False` in `.bazelrc` with zero per-crate opt-ins, deferred third-party
  rendering such as `blake3` `_bs`, and the hostile-`PATH` plus
  declared-tool contract in [Rust Generation](../generation/rust.md#build-scripts);
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Bindgen LLVM-22-vs-23 compat with fixture evidence qualified seed-only under #473
  (`bazel run //tools/ci:bindgen_qualification`; LLVM-22 parser baseline
  vs LLVM-23 target pinned in `rust/tests/fixtures/bindgen/pins.bzl`
  with the standalone/build-script `bindgen.h` plus `bindgen.expected`
  identical-set fixture pair, unpinned LLVM rejected, and the
  `--no-include-path-detection --formatter=none` plus execution-libclang
  plus target-flags plus separate-native-link contract in [Rust Generation](../generation/rust.md#binding-generation);
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- CXX graph identity with fixture evidence qualified seed-only under #474
  (`bazel run //tools/ci:cxx_identity_qualification`; single crate_universe
  `crates` graph with `cxx == cxxbridge-cmd == 1.0.200` and
  `@crates//:cxxbridge-cmd`, never a `cxx.rs` second graph, the
  `rust/tests/fixtures/cxx_identity/` Rust plus C++ composition under
  `bazel test //...`, and `dx generate --check` stability; mixed versions
  stay rejected; full `cxxbridge-cmd` execution plus corpus wiring stays
  owned under issue #499; platform plus consumer plus release evidence stays
  owned gap; no Supported claim).
- Exact-target discovery with fixture evidence qualified seed-only under #475
  (`bazel run //tools/ci:exact_target_qualification`; resolver-owned exact labels
  to upstream `gen_rust_project`/`flycheck` `TARGETS` with the hello exact-isolation
  pair (`src/lib.rs` to `hello_lib` only, `src/main.rs` to `hello` only) plus
  `rust/tests/fixtures/discovery/pins.bzl`, `Path`/`Buildfile` widening plus
  project-owned graph plus `RustAnalyzerInfo` rejected, and the discovery contract
  in [Target Resolution](../cli/target-resolution.md#exact-target-discovery);
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- JUnit 6.1.3 plus 5.14.x fallback with fixture evidence qualified seed-only under #476
  (`bazel run //tools/ci:junit_qualification`; JUnit 6.1.3 primary plus Jupiter 5.14.4
  plus Platform 1.14.4 fallback pinned in `java/tests/fixtures/junit/pins.bzl` with
  the Java/Kotlin Jupiter `execute --select-class` console-launcher fixture pair over
  `maven_install.json` plus fail-closed repin, JUnit 4.13.2 seed stays via Vintage
  (deprecated), unpinned runner rejected; `suspend` stays documented capability;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- Go test wiring with fixture evidence qualified seed-only under #478
  (`bazel run //tools/ci:gotest_qualification`; rules_go 0.63.0 plus Go SDK 1.26.6
  pinned in `go/tests/fixtures/gotest/pins.bzl` with the hello `go_test`
  package-level `embed` fixture over the `go_test` wrapper (upstream providers plus
  `QualitySourcesInfo`), Gazelle package-level `go_test` plus `embed` emission,
  implicit runner rejected; Go `from_file` qualified seed-only under #483;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- GoogleTest v1.18.0 plus C++17 floor with fixture evidence qualified seed-only under #479
  (`bazel run //tools/ci:googletest_qualification`; GoogleTest 1.18.0 pinned in
  `MODULE.bazel` plus `cc/tests/fixtures/googletest/pins.bzl` with the
  `cc_test` over `@googletest//:gtest_main` fixture plus explicit `-std=c++17`
  floor (`static_assert(__cplusplus)` plus `std::optional` plus
  structured-bindings plus `if constexpr`), plain assert seed stays, living at
  head rejected; platform plus consumer plus release evidence stays owned gap;
  no Supported claim).
- ScalaTest 3.2.20 with fixture evidence qualified seed-only under #480
  (`bazel run //tools/ci:scalatest_qualification`; rules_scala 7.3.0 plus Scala
  2.13.18 plus ScalaTest 3.2.20 pinned in `scala/tests/fixtures/scalatest/pins.bzl`
  with the hello `scala_test` `AnyFlatSpec` fixture over the `scala_test` wrapper
  (upstream providers plus `QualitySourcesInfo`), managed Coursier route with no
  Maven lock members, unpinned runner rejected; `scala_junit_test` plus
  `scala_specs2_junit_test` stay rules-supported choices, not defaults;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- Maven `maven_install.json` plus fail-closed repin with fixture evidence qualified seed-only under #481
  (`bazel run //tools/ci:maven_lock_qualification`; rules_jvm_external 7.1 plus
  `lock_file` plus `fail_if_repin_required` pinned in `third_party/jvm/pins.bzl` with
  the single shared lock over the `@maven` hub (JUnit 6.1.3 line plus 4.13.2 seed,
  per-artifact sha256, `REPIN=1 bazel run @maven//:pin` never hand-edited), proven by
  the Java/Kotlin Jupiter plus seed fixtures; non-fail-closed rejected
  (`maven_lock_qualification` 16/16); platform plus consumer plus release evidence stays
  owned gap; no Supported claim).
- Paket files plus sha512 with fixture evidence qualified seed-only under #482
  (`bazel run //tools/ci:paket_qualification`; `paket.dependencies` plus
  `paket.lock` via `paket2bazel` into the `paket.main` hub carrying
  per-package sha512, pinned in `csharp/tests/fixtures/paket/pins.bzl` with
  the F# hello `fsharp.core` consumer plus the xUnit hub consumers over
  `@paket.main`, generation consumes the Paket files and never writes them,
  NuGet native `packages.lock.json` rejected per rules_dotnet issue 444;
  per-platform SDK acquisition plus platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Go `go.mod`/`go.sum` via `go_deps.from_file` with fixture evidence qualified seed-only under #483
  (`bazel run //tools/ci:godeps_qualification`; `third_party/go/go.mod` plus `go.sum`
  via `go_deps.from_file(go_mod = ...)` into `@com_github_*` repos carrying go.sum
  verification, pinned in `go/tests/fixtures/godeps/pins.bzl` with the go-cmp fixture
  consumer over `@com_github_google_go_cmp//cmp:cmp` plus the exact `# gazelle:resolve`
  mapping; single-module layout with no `go.work`, hand `go_deps.module` tags rejected
  per Gazelle preferred; generation consumes the lock and never writes it;
  per-platform SDK acquisition plus platform plus consumer plus release evidence stays
  owned gap; no Supported claim (`godeps_qualification` 16/16)).
- C/C++ sha256-integrity plus no-system-package wiring with fixture evidence
  qualified seed-only under #484
  (`bazel run //tools/ci:cc_hermetic_qualification`; no ecosystem lockfile,
  every `http_archive` carries `sha256` or `integrity` pinned in
  `cc/tests/fixtures/hermetic/pins.bzl` with the committed `MODULE.bazel.lock`
  BCR integrity plus the depcheck `cc_deps.toml` plus `cc_lock.json` sha256
  pair, proven by the hello seed plus the GoogleTest mapping plus depcheck
  consistency and hash-less rejection; system packages rejected as non-hermetic
  (`cc_hermetic_qualification` 16/16); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- JVM quality defaults with fixture evidence qualified seed-only under #485
  (`bazel run //tools/ci:jvm_quality_qualification`; google-java-format 1.35.0
  plus Checkstyle 14.1.0 plus PMD 7.27.0 plus SpotBugs 4.10.4 plus ktfmt 0.63
  plus ktlint 1.8.0 plus detekt 1.23.8 plus Error Prone 2.50.0 pinned in
  `java/tests/fixtures/jvm_quality/pins.bzl` over upstream built-in defaults
  with no hidden preset, Checkstyle Google checks never auto-supplied, detekt
  `buildUponDefaultConfig` not `allRules`, beyond-default switches rejected,
  proven by the java/kotlin hello fixtures with no adapter claim; digests plus
  adapters stay owned under #416
  (`jvm_quality_qualification` 16/16); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Scala + .NET quality defaults with fixture evidence qualified seed-only under #486
  (`bazel run //tools/ci:scala_dotnet_defaults_qualification`; Scalafmt 3.11.4
  plus Scalafix 0.14.7 plus CSharpier 1.3.0 plus Fantomas 7.x stable plus
  FSharpLint 0.27.0 plus SDK-coupled Roslyn pinned in
  `scala/tests/fixtures/scala_dotnet_quality/pins.bzl` over upstream built-in
  defaults with no hidden preset, no auto-supplied OrganizeImports plus
  RemoveUnused preset, Roslyn SDK default analysis mode with StyleCop opt-in,
  FSharpLint default ruleset with formatting rules off, auto preset rejected,
  proven by the scala/csharp/fsharp hello fixtures with no adapter claim;
  digests plus adapters stay owned under #417
  (`scala_dotnet_defaults_qualification` 17/17); platform plus consumer plus
  release evidence stays owned gap; no Supported claim).
- Native quality defaults with fixture evidence qualified seed-only under #487
  (`bazel run //tools/ci:native_quality_qualification`; hermetic-llvm v0.8.19
  plus LLVM 23.1.0 toolchain for clang-format/clang-tidy plus cppcheck 2.21.0
  plus gofumpt v0.11.0 plus staticcheck 2026.2 plus govet following Go SDK 1.26.6
  plus errcheck v1.20.0 pinned in `cc/tests/fixtures/native_quality/pins.bzl`
  over upstream built-in defaults with no hidden preset, staticcheck default
  checks with the `SA`-only shortcut rejected, `govet` default analyzers with
  errcheck complementary, clang-tidy default checks, cppcheck default enablement,
  beyond-default switches rejected, proven by the cc/go hello fixtures with no
  adapter claim; digests plus adapters stay owned under #418
  (`native_quality_qualification` 17/17); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Structured quality defaults with fixture evidence qualified seed-only under #488
  (`bazel run //tools/ci:structured_defaults_qualification`; buf 1.72.0
  plus qmlformat/qmllint following the qualified Qt distribution pin (Qt
  6.11.2 plus 6.11.1 observed) pinned in
  `quality/tests/fixtures/structured_quality/pins.bzl` over upstream
  built-in defaults with no hidden preset, `buf` `STANDARD` as the upstream
  built-in default lint set with native ini interpretation for qml, no
  auto-supplied buf.yaml or ini preset, beyond-default `COMMENTS` plus
  `UNARY_RPC` opt-in maxima rejected, proven by the fixture pair with no
  adapter claim; digests (including Qt distribution identity, licensing,
  and platform artifacts) plus adapters stay owned under #419
  (`structured_defaults_qualification` 17/17); platform plus consumer plus
  release evidence stays owned gap; no Supported claim).
- Non-dogfed execution plan delivered (see issue #508 for remaining gaps; hermetic
  CLI-contract pins under issue #407)
  (`bazel run //tools/ci:non_dogfed_paths`; hermetic CLI-contract pins,
  green hermetic failure proofs (issue #406), coverage-excluded runs, shell ownership
  plus test execution with no quality class by design).

The consumer aggregate `dx-ci`
([contract](../github-ci.md#aggregate-status)) is unchanged: stable identity,
disabled-as-skipped stays green, any other non-success fails.
