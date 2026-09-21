# Verification matrix — remaining reds (split from `verification-matrix.md`). No behavior change.

See [`verification-matrix.md`](./verification-matrix.md) for Layers/Status/Battery.

Remaining reds stay owned gaps, not green claims:
- Per-cell coverage is qualified for the seed plus arm64 plus two static-musl plus macos arm64 plus macos x86_64 best-effort plus windows x86_64 cells with fixture evidence under
  #507 (`tools/coverage/tests/fixtures/per_cell/pins.bzl` plus
  `per_cell.expected` plus `codecov_remote.expected` via
  `bazel run //tools/ci:coverage_qualification`; seven cells, same
  scope, no union, Starlark fallback, Codecov opt-in, quotas, local-only
   remote evidence). First-party PR reporting is
   adopted under #254 (Codecov opt-in only; the seed cell owns the PR comment,
   the arm64 plus musl plus macos plus macos-x86_64 plus windows cells report to their job summaries; the macos x86_64 best-effort cell reports without blocking required-host release). All required plus best-effort cells are qualified; out-of-v1 hosts stay platform-gated under closed #298.
- Docs pipeline delivered seed-only plus environment/codegen stays open under #787 plus onboarding #788 (see
   [Documentation](../documentation/README.md#contracts); successors to closed #581 and #506; adapter runs delivered under #779, renderer/site execution delivered seed-only under #780, rebuild proof delivered seed-only under #781, link/reference completeness delivered seed-only under #782, guide-step wiring delivered seed-only under #783, first-hour timing proof delivered seed-only under #784, per-release pin-bump plus drift process delivered seed-only under #785). Environment/codegen
   deferred records plus fixture evidence are qualified seed-only under closed #506
  (`bazel run //tools/ci:env_codegen_qualification` with
  `env/tests/fixtures/env_codegen/pins.bzl` plus `env_codegen.expected`
  plus `roots_bep.txt`; no junction/copy fallback, no checksum-only
  fallback, no third-party plugin claim; bootstrap, fidelity, spaces,
  stale-clean, IDE, atomic-commit, BEP, projection, root-candidate, and
  cold-warm qualified with WP shard plus root plus collector evidence;
  platform plus consumer plus release evidence qualified under #787 with
  per-required-host symlink-only plus refusal, adopt-consumer, and release
  checklist linkage; admitted-pairs evolution onboarding qualified under #788
  with checklist plus per-pair fixtures plus qualification coverage for each
  admitted pair; #751 plus #752 plus #753 stay open and out of scope
  for #787 plus #788; no Supported
  claim). Docs-pipeline IR plus
  planning plus adapter runs plus site-execution plus rebuild plus link plus guide plus timing plus drift records with fixture evidence are qualified seed-only under #779
  plus #780 plus #781 plus #782 plus #783 plus #784 plus #785
  (live successor to closed #421)
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR, codec,
  planning, frozen contracts, removed stub, per-language adapter runs with pins
  plus golden fixtures plus Bazel-cached extract to render with
  mdBook-compatible prose plus API pages plus one search index and generated IR in Bazel
  outputs only plus byte-identical rebuild proof plus link/reference completeness with no dangling targets plus guide-step CI wiring with every step executed and no unexecuted steps plus first-hour timing proof one-shot per ADR 0022 with no CI timing budget plus per-release pin-bump plus drift process with no IR snapshot update; no owned gaps remain; no working site claimed).
- Consumer-CI contract plus caller plus gate/aggregate plus per-gap decisions
  with fixture evidence qualified seed-only under #509
  (`tools/ci/tests/fixtures/consumer_ci/pins.bzl` plus `platforms.expected`
  plus `revisions.expected` plus `reporting.expected` via
  `bazel run //tools/ci:consumer_ci_qualification`; nine checks, explicit
  platforms, fail-closed sequential, stable dx-ci aggregate, hygiene,
  concurrency, permissions, per-cell coverage with fork-safe comments,
  self-call test-disabled (issue #408 plus Phase 1 #607 coverage superset,
  verbatim `//...`), native bump loop
  (sole updater, issue #461),
   dx migrate syntax plus manifest selection (delivered CLI with fail-closed
   execution, issue #462) plus dx run multirun (issue #463 delivered), tag
   hygiene as-built; Consumer-CI per-gap decisions with fixture evidence:
   platform, runner, isolation, cache, ordering, merge,
   diff, queue, cancellation, aggregate binding, thread identity, ordering,
   limits (frozen at 50 under issue #592), fork, untrusted, sensitive, retries,
   Code-Scanning, sequential,
   tag/release, native-bot, migrate-manifest decisions pinned with
   build-only self-call forever rejected per #408; platform plus release
   evidence stays owned gap; no Supported claim;
   `consumer_ci_qualification` 43/43);
- Review-thread limit plus accounting frozen at 50 open threads with fixture
  evidence qualified seed-only under issue #592
  (`tools/ci/tests/fixtures/review_threads/pins.bzl` plus
  `review_threads.expected` via `bazel run //tools/ci:review_threads_qualification`;
  no consumer setting, no fresh allowance, failure-first check/file/line/rule
  ordering, no rotation, bot-only frees with resolved history outside the open
  count, full reports with summary-distinguished truncation, no overwrite;
  leaving unfrozen plus consumer setting plus fresh allowance plus completion
  order plus rotation rejected; CI-only, no Supported claim;
  `review_threads_qualification` 16/16);
   platform qualification
   beyond the seed plus arm64 plus musl plus macos plus macos-x86_64 plus
   windows hosts stays open under #803-#807 (successors to closed #298; native
   qualification delivered seed-only under closed #410-#414).
- File-family quality record with fixture evidence qualified seed-only under #489
  (`bazel run //tools/ci:file_family_qualification`; provider-class
  applicability with never-suffix inference, Starlark/Buildifier plus TOML/Taplo
  adapter-backed evidence, parity-deferred CSS/djlint/buf/yaml/keep-sorted/shell
  plus cue/jsonnet/pkl/qml/terraform routes with owner plus frozen acquisition;
  deferred adapter execution, exact pins/digests/rule-sets/mappings, and platform
  plus consumer plus release evidence stay owned gaps under #796-#800 (successors
  to closed #416-#420); no Supported claim).
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
   parsing with auto help stays owned gap under #810).
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
  stay rejected; full `cxxbridge-cmd` execution plus corpus wiring qualified
  seed-only under issue #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via
  `bazel run //tools/ci:linux_corpus_qualification`); platform plus consumer
  plus release evidence stays owned gap; no Supported claim).
- Exact-target discovery with fixture evidence qualified seed-only under #475
  (`bazel run //tools/ci:exact_target_qualification`; resolver-owned exact labels
  to upstream `gen_rust_project`/`flycheck` `TARGETS` with the hello exact-isolation
  pair (`src/lib.rs` to `hello_lib` only, `src/main.rs` to `hello` only) plus
  `rust/tests/fixtures/discovery/pins.bzl`, `Path`/`Buildfile` widening plus
  project-owned graph plus `RustAnalyzerInfo` rejected, and the discovery contract
  in [Target Resolution](../cli/target-resolution.md#exact-target-discovery);
  platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- C++ exact-target snapshot with fixture evidence qualified seed-only under #754
  (`bazel run //tools/ci:cpp_snapshot_qualification`; resolver-owned exact labels
  as snapshot input with the hello exact-isolation pair (`hello.cc` to `hello_lib`
  only, `main.cc` to `hello` only) plus `cc/tests/fixtures/cpp_snapshot/pins.bzl`
  plus `snapshot.expected` plus `exact_targets.txt`, action-derived `CppCompile`
  via `aquery` with command plus inputs, generated-output materialization through
  exact mappings, `hello.h` multi-context kept apart, managed host-native clangd
  never unrestricted query-driver, Bazel 9.2.0 plus rules_cc 0.2.22, `CcInfo`
  inference plus workspace-writing refresh plus continued-after-failures plus
  package-wide widening rejected, and the snapshot contract
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
  adapters stay owned under #796 (successor to closed #416)
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
  digests plus adapters stay owned under #797 (successor to closed #417)
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
  adapter claim; digests plus adapters stay owned under #798 (successor to closed #418)
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
  and platform artifacts) plus adapters stay owned under #799 (successor to closed #419)
  (`structured_defaults_qualification` 17/17); platform plus consumer plus
  release evidence stays owned gap; no Supported claim).
- File-family quality defaults with fixture evidence qualified seed-only under #489
  with modfmt plus gherkin/xml resolved seed-only under #582
  (`bazel run //tools/ci:file_family_defaults_qualification`; cue v0.17.1
  plus jsonnetfmt v0.22.0 plus pkl 0.32.1 plus terraform v1.16.1 plus djlint
  v1.45.0 plus Stylelint 17.14.1 plus Prettier 3.9.6 with prettier-plugin-sql
  0.15.1 plus prettier-plugin-gherkin 4.0.0 plus @prettier/plugin-xml 3.4.2 plus modfmt v0.4.0
  plus yamlfmt v0.21.0 plus yamllint 1.38.0 plus keep-sorted v0.10.0
  pinned in `quality/tests/fixtures/file_family_quality/pins.bzl` over upstream
  built-in defaults with no hidden preset, whole-file rewrite versus check-only
  per tool with no auto-supplied preset, suffix inference rejected with
  registry-owned applicability, beyond-default switches rejected, proven by the
  fixture pair with no adapter claim; digests plus adapters stay owned under #800 (successor to closed #420);
  `protobuf`/`qml` stay owned by #799 (successor to closed #419), never double-claimed
  (`file_family_defaults_qualification` 17/17); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Scalafix console plus semanticdb-classpath wiring with fixture evidence
  qualified seed-only under issue #490
  (`bazel run //tools/ci:scalafix_qualification`; console-parse is rejected
  and wire via `scalafix.interfaces.ScalafixMainCallback` is required with
  target-coupled `--classpath` plus `--sourceroot` plus
  `--semanticdb-targetroots` plus sandbox-apply-and-diff with declared
  outputs, silent console parse rejected, pinned in
  `scala/tests/fixtures/scalafix/pins.bzl` with `console_lint.txt` plus
  `console_rewrite.txt` lossiness proof, proven by the scala hello fixture
  with no adapter claim; digests plus `scala` adapter stays owned under #797 (successor to closed #417)
  (`scalafix_qualification` 12/12); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Roslyn per-TFM-RID SARIF aggregation with fixture evidence qualified
  seed-only under issue #492
  (`bazel run //tools/ci:roslyn_qualification`; per-pivot `/errorlog`
  SARIF 2.1 runs concatenate into one log with a single schema plus version
  in deterministic pivot order, union of results with
  per-pivot provenance, single-SARIF assumption plus merged-single-run
  rejected, declared inputs with fail-closed, pinned in
  `csharp/tests/fixtures/roslyn/pins.bzl` with `net8.sarif` plus
  `net10.sarif` union proof (shared CA1822 in both, TFM-specific CA1303
  only in net10) plus `aggregated.sarif`, proven by the csharp hello fixture
  with no adapter claim; digests plus `csharp` adapter stays owned under #797 (successor to closed #417)
  (`roslyn_qualification` 13/13); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- FSharpLint console vs library-API binding with fixture evidence
  qualified seed-only under issue #493
  (`bazel run //tools/ci:fsharplint_qualification`; console-parse is
  rejected both shapes and wire via `FSharpLint.Application.Lint` with
  `ReceivedWarning` is required with target-coupled `.fsproj`/`.sln` plus
  `fsharplint.json` wiring plus sandbox-apply-and-diff with declared
  outputs, silent console parse rejected, pinned in
  `fsharp/tests/fixtures/fsharplint/pins.bzl` with `console_standard.txt`
  plus `console_msbuild.txt` lossiness proof (standard blocks carry
  start-only locations with URL-only rule IDs plus shared-stream info
  lines, `-f msbuild` lines carry full ranges but drop fix plus typecheck
  context) plus `example.fsharplint.json` (formatting-adjacent rules off,
  Fantomas owns formatting), proven by the fsharp hello fixture with no
  adapter claim; digests plus `fsharp` adapter stays owned under #797 (successor to closed #417)
  (`fsharplint_qualification` 13/13); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Windows immutable-lazy acquisition with fixture evidence qualified seed-only under #495
  (`bazel run //tools/ci:windows_acquisition_qualification`;
  fixed-manifest plus package-index inputs pinned in
  `cc/tests/fixtures/windows_acquisition/pins.bzl` with the toolchains_msvc
  head plus windows_support `v0.4.1` package identities, mutable fetch
  rejected, laziness proven with missing acceptance plus unrelated workflows
  green and deferred failure never proving laziness; backend stays
  provisional; acquisition rights qualified seed-only under #496, prebuilt interop
  qualified seed-only under #498, linux corpus qualified seed-only under #499,
  windows transport qualified seed-only under #497, floors qualified seed-only
  under #500, coverage qualified seed-only under #501;
  platform plus consumer plus release evidence stays owned gap; no Supported
  claim (`windows_acquisition_qualification` 14/14)).
- Apple plus Microsoft acquisition rights with fixture evidence qualified seed-only under #496
  (`bazel run //tools/ci:acquisition_rights_qualification`;
  hermetic-llvm `v0.8.19` MacOSX26.5 extraction vs Apple SDK agreement with
  hosted execution not authorizing separate extraction or unrestricted caching,
  toolchains_msvc head plus windows_support `v0.4.1` identities with deliberate
  EULA repository-env never automatic plus SDK EULA gap reviewed not inferred,
  usage vs redistribution reviewed separately with acceptance not redistribution
  permission, mirrors plus redistribution plus internal caches plus remote workers
  as license-approved boundaries separately from download success, official download
  not permission, assume rights rejected, pinned in
  `cc/tests/fixtures/acquisition_rights/pins.bzl`; backends stay provisional;
  prebuilt interop qualified seed-only under #498, linux corpus qualified
  seed-only under #499, windows transport qualified seed-only under #497,
  floors qualified seed-only under #500, coverage qualified seed-only under
  #501; platform plus consumer plus release evidence
  stays owned gap; no Supported claim (`acquisition_rights_qualification` 15/15)).
- Windows transport plus ABI with fixture evidence qualified seed-only under #497
  (`bazel run //tools/ci:windows_transport_qualification`;
  explicit ABI constraint fix pinned in
  `cc/tests/fixtures/windows_transport/pins.bzl` with the toolchains_msvc
  head plus rules_rs `v0.0.109` LLVM MSVC ABI identity (GNU/GNULVM rejected),
  `/external:I` plus `.lib`/`.obj` bare-path handling with the patched
  rules_rust rebasing baseline, batch wrappers plus response-file contents
  plus spaces plus SDK libraries plus cc-rs discovery/assembly plus
  proc-macro DLLs on declared-input fixtures without host Visual Studio
  state, compiler-target availability alone is not proof; backend stays
  provisional; rights plus interop qualified seed-only under #496/#498,
  linux corpus qualified seed-only under #499, floors qualified seed-only
  under #500, coverage qualified seed-only under #501;
  platform plus consumer plus release evidence stays owned gap; no Supported
  claim (`windows_transport_qualification` 16/16)).
- Prebuilt interop with fixture evidence qualified seed-only under #498
  (`bazel run //tools/ci:prebuilt_interop_qualification`;
  explicit Windows STL/CRT/linker/library combos with clang-cl plus Microsoft STL
  plus retail `/MD` plus static `.lib` plus import `.lib` plus DLL shapes plus
  `cl.exe` compat as diagnosis only plus Linux explicit dynamic libstdc++
  comparison on Linux glibc only plus ordinary object linking without
  cross-language LTO plus shared-runtime plus exceptions plus RTTI plus
  allocation-ownership plus ABI boundaries, host-to-target plus target execution
  separately with mixed Rust/C/C++ composition, compiler-target availability alone
  plus LLVM ancestry alone plus single-combo proof rejected, pinned in
  `cc/tests/fixtures/prebuilt_interop/pins.bzl` with the interop lib plus test
  under `bazel test //...`; backends stay provisional; linux corpus qualified
  seed-only under #499, windows transport qualified seed-only under #497,
  floors qualified seed-only under #500, coverage qualified seed-only under
  #501; platform plus
  consumer plus release evidence stays owned gap; no Supported claim
  (`prebuilt_interop_qualification` 16/16)).
- Linux corpus with fixture evidence qualified seed-only under #499
  (`bazel run //tools/ci:linux_corpus_qualification`;
  source-built SQLite plus OpenSSL with declared build tools plus ring-style
  C/assembly with target libs plus bindgen standalone plus build-script routes
  with execution libclang closure plus CXX single-graph execution, native only
  with single-crate proof rejected, pinned in
  `cc/tests/fixtures/linux_corpus/pins.bzl` with the corpus lib plus test
  under `bazel test //...`; backends stay provisional; windows transport
  qualified seed-only under #497, floors qualified seed-only under #500,
  coverage qualified seed-only under #501; platform plus
  consumer plus release evidence stays owned gap; no Supported claim
  (`linux_corpus_qualification` 16/16)).
- Deployment plus execution floors with fixture evidence qualified seed-only under #500
  (`bazel run //tools/ci:deployment_floors_qualification`;
  glibc `2.28` symbol floor plus musl `1.2.6` static closure plus macOS
  deployment `14.0` plus MacOSX26.5 SDK identity plus Windows retail `/MD`
  CRT with Microsoft STL/UCRT/VCRuntime plus MSVC `14.50.35717` plus redist
  `14.50.35710` plus SDK package `10.0.26100.7705` identities, oldest-target
  and current-host fixtures run separately with matching native target
  execution, compiler plus clangd plus bindgen loader dependencies inspected
  separately, Apple extracted SDK framework subset checked, SDK version is not
  deployment floor, `/MT` plus debug CRT not interchangeable, unpinned floors
  rejected, pinned in `cc/tests/fixtures/deployment_floors/pins.bzl` with
  `floors.expected` plus `oldest_target.txt` plus `current_host.txt` plus
  `loader_deps.txt` plus `apple_frameworks.txt`; backends stay provisional;
  linux corpus qualified seed-only under #499, coverage qualified seed-only
  under #501; platform plus consumer plus release evidence stays owned gap;
  no Supported claim (`deployment_floors_qualification` 16/16)).
- LCOV accounting with fixture evidence qualified seed-only under #501
  (`bazel run //tools/ci:lcov_accounting_qualification`;
  Rust-only plus C/C++-only plus mixed/DLL LCOV with missed-line tests,
  coverage-tool version pairing, native ignores and denominator validation,
  pinned in `cc/tests/fixtures/lcov_accounting/pins.bzl` with the accounting
  lib plus test under `bazel test //...`; unaccounted lines plus ignored
  collection failures rejected; backends stay provisional; floors qualified
  seed-only under #500; platform plus consumer plus release evidence stays
  owned gap; no Supported claim (`lcov_accounting_qualification` 16/16)).
- Cargo metadata with fixture evidence qualified seed-only under #502
  (`bazel run //tools/ci:cargo_metadata_qualification`;
  features plus build-script metadata plus target kinds plus ownership without
  private serialized dependency-graph access with narrow upstream exports where
  missing, pinned in `rust/tests/fixtures/cargo_metadata/pins.bzl` with the
  `Cargo.toml` plus `cargo_metadata.expected` pair plus hello plus cc_optout
  plus cargo testdata shapes under `bazel test //...`; ad-hoc metadata rejected;
  backends stay provisional; floors qualified seed-only under #500, coverage
  qualified seed-only under #501; platform plus consumer plus release evidence
  stays owned gap; no Supported claim (`cargo_metadata_qualification` 16/16)).
- Strict generation with fixture evidence qualified seed-only under #503
  (`bazel run //tools/ci:strict_generation_qualification`;
  quoted basename identities resolving strictly or failing plus angle
  includes never producing edges plus ambiguous basenames failing plus macro
  includes contributing no edge with no synthesized ignore plus inert
  comments and literals plus authoritative local-index plus exact-mapping
  metadata never the module index plus generated headers requiring exact
  mappings plus test grouping keeping `*_test` out plus undiscovered assembly
  plus handwritten modules and header units and PCH plus union of literals
  with deterministic names plus merge lifecycle, pinned in
  `cc/tests/fixtures/strict_generation/pins.bzl` with the strict lib plus
  test under `bazel test //...` plus `//gazelle/cc:cc_test` plus
  `//gazelle/cc:generation_test`; loose generation plus first-candidate plus
  warn-and-omit plus disappearing angle plus log parsing plus replacement
  preprocessor plus flags-as-support plus inferred producers plus platform
  selections rejected; backends stay provisional; floors qualified seed-only
  under #500, coverage qualified seed-only under #501, linux corpus qualified
  seed-only under #499; platform plus consumer plus release evidence stays
  owned gap; no Supported claim (`strict_generation_qualification` 16/16)).
- Cross routes with fixture evidence qualified seed-only under #504
  (`bazel run //tools/ci:cross_routes_qualification`;
  Linux x86_64 plus Linux arm64 execution each covering Linux x86_64 and
  arm64 glibc plus static musl as the first cohort, not a mandate to build
  every target from every host, with native rows qualified under #410-#414
  and same-arch musl closures qualified under #411 plus Linux cross-arch
  staying in the cohort with matching native target execution plus separate
  cache and remote evidence plus macOS arm64 to Linux plus macOS x86_64 as
  optional expansion only if bounded upstream configuration suffices plus
  Linux/macOS-to-Windows plus Linux/Windows-to-macOS plus Windows-to-Linux
  plus Windows arm64 excluded without weakening required native workflows,
  pinned in `cc/tests/fixtures/cross_routes/pins.bzl` with
  `routes.expected` plus `execution.txt` plus `cache_remote.txt`;
  all-cross mandate plus cross-building alone plus emulation plus Rosetta
  plus remote as execution plus compiler-target availability plus cross-host
  Windows inference rejected; backends stay provisional; floors qualified
  seed-only under #500, coverage qualified seed-only under #501, linux
   corpus qualified seed-only under #499; platform plus consumer plus release
  evidence stays owned gap; no Supported claim
  (`cross_routes_qualification` 17/17)).
- Bounded remediation with fixture evidence qualified seed-only under #505
  (`bazel run //tools/ci:remediation_bounds_qualification`;
  every native defect reproduced with estimate plus actual owner plus patch
  plus upstream-issue plus upgrade tracking plus complete-workflow evidence
  with focused tested pinned patches plus bounded integration plus small
  missing pieces with clearly bounded scope plus upstreaming preferred plus
  established-alternative comparison with explicit contract plus API review
  plus PIE plus ELF-dependency plus glibc-symbol via existing upstream
  constraints plus cross-product expansion only after the cohort passes plus
  stop-slice on replacement acquisition engine plus compiler backend plus
  Cargo graph plus coverage engine with no deferred Rust Windows support plus
  no premature C/C++ admission plus no weakened hermeticity, pinned in
  `cc/tests/fixtures/remediation_bounds/pins.bzl` with `bounds.expected`
  plus `defects.txt` plus `owners.txt`; Do not implement missing infra
  merely to fill cross-product; unbounded fork plus whole rewrite plus
  replacement engine plus owned backend plus cargo-zigbuild plus installed
  fallback plus cross-product infra fill plus weakened natives rejected;
  backends stay provisional; floors qualified seed-only under #500, coverage
  qualified seed-only under #501, linux corpus qualified seed-only under
  #499, routes qualified seed-only under #504; platform plus consumer plus
  release evidence stays owned gap; no Supported claim
  (`remediation_bounds_qualification` 16/16)).
- Layer-2 adapter-less plus composition plus depcheck with fixture evidence
   qualified seed-only under closed #510
   (`bazel run //tools/ci:layer2_opens_qualification`;
  Go plus Java plus Kotlin plus Scala plus C# plus F# plus C++ Layer-2
  Open adapter-less with no adapter claim plus no runner-matrix cells plus
  parity deferred, Vue plus Svelte plus Astro plus MDX regions
  classification-only with no adapter claim plus no matrix cells plus no
  curated defaults, composition evidence in `examples/mixed/hello/` plus
  `gazelle/mixed/` with one wrapper per container plus shared helper plus
  no framework-to-framework imports plus disjoint partition, Depcheck Open
  for frameworks with framework-composition depcheck staying with the JS/TS
  pnpm route, required-core plus admitted fixtures delivered in
  `tools/depcheck/` under #22, pinned in
  `quality/tests/fixtures/layer2_opens/pins.bzl` with
   `layer2_opens.expected`; Closed #303 only, #796-#800 adapters (successors to
   closed #416-#420) with digests plus adapters staying owned, adapter-less as pass rejected;
  backends stay provisional; platform plus consumer plus release evidence
  stays owned gap; no Supported claim
  (`layer2_opens_qualification` 16/16)).
- Per-cell non-seed coverage plus Codecov opt-in plus remote evidence
  with fixture evidence qualified under #507
  (`bazel run //tools/ci:coverage_qualification`;
  seven required plus best-effort cells gating their own combined LCOV
  report separately with the same first-party scope and no cross-cell
  union, 100% non-ignored exact gate in this repo with a configurable
  `--min-coverage` requirement for users per-cell, Codecov opt-in only
  never required with no activation or upload wiring, standard-runner
  free-tier quotas with paid routes banned, local-only remote with
  locally sandbox-tested hermeticity and remote behavior unverified,
  pinned in `tools/coverage/tests/fixtures/per_cell/pins.bzl` with
  `per_cell.expected` plus `codecov_remote.expected`;
  seed-only-forever plus cross-cell union plus averaged percentages plus
  rounding up plus required Codecov plus remote-correctness claims
  rejected; floors qualified seed-only under #500, LCOV accounting
  qualified seed-only under #501; platform plus consumer plus release
  evidence stays owned gap; no Supported claim
  (`coverage_qualification` 33/33)).
- Stable-stack compose with fixture evidence qualified seed-only under issue #494
  (`bazel run //tools/ci:stable_stack_qualification`; as-built Bzlmod
  identities Bazel 9.2.0 plus rules_rust 0.74.0 plus rules_cc 0.2.22 plus
  Rust 1.98.0 with `MODULE.bazel.lock` integrity, rules_rs LLVM `0.8.18`/
  LLVM `22.1.8` baseline vs hermetic-llvm `0.8.19`/LLVM `23.1.0` candidate
  comparison, checksums plus source patches plus compiler/profile
  compatibility, ad-hoc compose rejected, pinned in
  `rust/tests/fixtures/stable_stack/pins.bzl`, proven by the Rust plus C++
  hello fixtures with no adapter claim; linux corpus qualified seed-only under
  #499 (`cc/tests/fixtures/linux_corpus/pins.bzl` via
  `bazel run //tools/ci:linux_corpus_qualification`, single-crate proof
  rejected, native only) with the candidate backend provisional
  (`stable_stack_qualification` 16/16); platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Non-dogfed execution plan qualified seed-only under #508
  (`bazel run //tools/ci:non_dogfed_qualification` with
  `tools/ci/tests/fixtures/non_dogfed/pins.bzl` plus `non_dogfed.expected`;
  `non_dogfed_qualification` 16/16; hermetic CLI-contract pins under issue #407)
  (`bazel run //tools/ci:non_dogfed_paths`; hermetic CLI-contract pins,
  green hermetic failure proofs (issue #406), coverage-excluded runs, shell ownership
  plus test execution with no quality class by design).
- Quality family taxonomy execution with fixture evidence qualified seed-only under closed #512
  (`bazel run //tools/ci:quality_taxonomy_qualification` with
  `quality/tests/fixtures/quality_taxonomy/pins.bzl` plus
  `quality_taxonomy.expected`; `quality_taxonomy_qualification` 17/17;
  47 classes each with exactly one owning family across 39 families with
  css/json/python/typescript/javascript/cc groupings, 8 curated families
  with lazy defaults plus 11 backed classes over 16 adapters with matrix
  plus parser plus native plus aspect plus policy execution, 36 deferred
  with ADR 0019 owner plus frozen route, curated audit empty with Bandit
  excluded plus secrets via Gitleaks, suffix rejected with cross-family
   union plus lazy plus no hidden preset; taxonomy doc only plus
    report-not-gate shape only rejected; deferred adapters owned under
    #796-#800 (successors to closed #416-#420) plus closed #307, digests plus rule-sets owned by cohorts, platform plus
    consumer plus release evidence stays owned gap under #802 and #808; backends provisional; no
    Supported claim; closed #512 stays taxonomy-only).
- Python source-audit split with fixture evidence qualified seed-only
    under closed #613
   (`bazel run //tools/ci:python_audit_qualification` with
   `python/tests/fixtures/python_audit/pins.bzl` plus
   `python_audit.expected`; `python_audit_qualification` 16/16;
   curated audit empty with Bandit excluded plus secrets via Gitleaks,
   lint Ruff plus pydoclint plus format Ruff plus typecheck Ty
   unaffected with flake8 plus pylint opt-ins, no audit adapter claim,
   source audit distinct from ecosystem audit/update delivered, leaving
    under taxonomy rejected with mismatched scope; future selection (#801) plus
    platform plus consumer plus release (#808) evidence stays owned gap;
    backends provisional; no Supported claim).
- Selective `dx update` per-set support with fixture evidence qualified
  seed-only under #583
  (`bazel run //tools/ci:selective_update_qualification` with
  `cli/update/tests/fixtures/selective_update/pins.bzl` plus
  `selective_update.expected`; `selective_update_qualification` 16/16;
  npm selective supported via the Bazel-pinned pnpm, Cargo/Maven/NuGet/Go
  selective wont-fix with whole-lock repin plus empty-Go no-op, silent
   full-update substitution plus private emulation rejected, bump follow-up
   automatic and resolver-owned; update-only, no lock format change; platform
   plus consumer plus release evidence stays owned gap; no Supported claim).
- Selective Cargo per-crate update with fixture evidence qualified
  seed-only under #633
  (`bazel run //tools/ci:selective_cargo_qualification` with
  `cli/update/tests/fixtures/selective_cargo/pins.bzl` plus
  `selective_cargo.expected`; `selective_cargo_qualification` 16/16;
  `cargo:<crate>` parses then fails closed as `unsupported` with the
  `dx update cargo` hint, whole-lock `crate_universe` repin only, silent
  full-update substitution plus private `cargo update -p` rejected, bump
  follow-up automatic and resolver-owned; update-only, no lock format change;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- Selective NuGet per-package update with fixture evidence qualified
  seed-only under #635
  (`bazel run //tools/ci:selective_nuget_qualification` with
  `cli/update/tests/fixtures/selective_nuget/pins.bzl` plus
  `selective_nuget.expected`; `selective_nuget_qualification` 16/16;
  `nuget:<id>` parses then fails closed as `unsupported` with the
  `dx update nuget` hint, whole-folder `paket2bazel` regen only, silent
  full-update substitution plus private `paket.lock` surgery rejected, bump
  follow-up automatic and resolver-owned; update-only, no lock format change;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
<- Selective Maven `dx update` per-artifact wont-fix with fixture evidence qualified seed-only under #634
  (`bazel run //tools/ci:selective_maven_qualification` with
  `cli/update/tests/fixtures/selective_maven/pins.bzl` plus
  `selective_maven.expected`; `selective_maven_qualification` 16/16;
  `maven:group:artifact` parses but stays unsupported with the
  `dx update maven` hint, whole-lock `REPIN=1 bazel run @maven//:pin`
  stays the only approved updater, silent full substitution plus private
  per-artifact emulation plus hand-edited `maven_install.json` rejected;
  update-only, no lock format change; platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Go real workspace (full no-op plus selective wont-fix) with fixture evidence qualified
  seed-only under #636
  (`bazel run //tools/ci:selective_go_qualification` with
  `cli/update/tests/fixtures/selective_go/pins.bzl` plus
  `selective_go.expected`; `selective_go_qualification` 16/16;
  full is an intentional no-op success (main workspace has no `go.mod` by
  design, pinned `go_deps.from_file` lock tracks Gazelle, no launch),
  `go:<module-path>` parses then fails closed as `unsupported` with the
  `dx bump gomod:<module> <version>` hint, silent full-update substitution
  plus private `go get` plus `go mod tidy` rejected, bump follow-up automatic
  and resolver-owned; update-only, no lock format change; platform plus
  consumer plus release evidence stays owned gap; no Supported claim).
- Bump Maven plus NuGet widen-one with unit evidence qualified seed-only
  under #637
  (`bazel test //cli/bump:dx_bump_test //cli/cli:dx_cli_test` bump filter;
  `maven:group:artifact` widens one `maven.install` artifact in
  `MODULE.bazel` then `dx update maven` whole-lock pin,
  `nuget:<id>` widens one `nuget <id> <version>` line in
  `third_party/dotnet/paket.dependencies` then `dx update nuget`
  whole-folder regen, missing/ambiguous shapes fail closed with nothing
  widened, silent batch substitution rejected; update-only, no lock format
  change; platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Bump then update chaining with fixture evidence qualified
  seed-only under #638
  (`bazel run //tools/ci:bump_chain_qualification` with
  `cli/bump/tests/fixtures/bump_chain/pins.bzl` plus
  `bump_chain.expected`; `bump_chain_qualification` 16/16;
  Cargo widen then automatic full repin, npm widen then automatic selective
  refresh, Go widen then automatic noop, Maven widen then automatic full pin,
  NuGet widen then automatic full regen, Bazel/GHA file-only with no launch,
  refresh failures keep the widen with `update_failed` and no rollback,
  manual second step plus private resolver rejected; bump plus refresh only,
  no lock format change; platform plus consumer plus release evidence stays
  owned gap; no Supported claim).
- Bump discovery outdated enumeration with fixture evidence qualified
  seed-only under #639
  (`bazel run //tools/ci:bump_discovery_qualification` with
  `cli/bump/tests/fixtures/bump_discovery/pins.bzl` plus
  `bump_discovery.expected`; `bump_discovery_qualification` 16/16;
  scheduled discovery enumerates outdated via upstream registry clients
  (BCR / crates.io / npm registry / Go proxy / Maven Central / NuGet /
  GitHub releases, never custom HTTP), stable only with prerelease following
  upstream plus semver ordering plus resolver-governed transitives, GHA tags
  need SHA resolution, up-to-date has no candidate, manual selector only plus
  custom HTTP plus private resolver rejected; automation only, no lock format
  change; platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Bump GHA tag-to-SHA auto resolution with fixture evidence qualified
  seed-only under #640
  (`bazel run //tools/ci:bump_gha_qualification` with
  `cli/bump/tests/fixtures/bump_gha/pins.bzl` plus
  `bump_gha.expected`; `bump_gha_qualification` 16/16;
  tags auto-resolve to SHA via the upstream GitHub releases client
  (`gh api`, never custom HTTP; planned in `dx_bump::gha`), shapes via
  `version::parse` (GitTag plus 40/64-char GitCommit), one tag per run with
  unknown tags failing closed and never an invented SHA, direct tags need SHA
  resolution before the file edit, manual SHA only plus custom HTTP plus
  invented SHA plus private resolver rejected; automation only, no lock format
  change; platform plus consumer plus release evidence stays owned gap; no
  Supported claim).
- Runner plus SDK rotation with fixture evidence qualified seed-only under #642
  (`bazel run //tools/ci:runner_rotation_qualification` with
  `tools/ci/tests/fixtures/runner_rotation/pins.bzl` plus
  `runner_rotation.expected`; `runner_rotation_qualification` 16/16;
  ubuntu-latest plus ubuntu-24.04-arm plus macos-14 plus macos-15-intel plus
  windows-latest with per-profile cache scopes, macos-13 retired December 2025
  plus macos-15-intel until August 2027, quarterly plus on retirement notice
  plus on hermetic-llvm release with sole-maintainer owner, SDK plus floor scope
  glibc 2.28 plus musl 1.2.6 plus MacOSX26.5 via hermetic-llvm v0.8.19 plus
  MSVC identities, customer-flows-only build/test/coverage with no new CI job,
  permanent rotation job rejected; infra only, no Supported claim).
- GHCR rebuild plus signing rotation with fixture evidence qualified seed-only under #647
  (`bazel run //tools/ci:ghcr_rebuild_rotation_qualification` with
  `tools/ci/tests/fixtures/ghcr_rebuild_rotation/pins.bzl` plus
  `ghcr_rebuild_rotation.expected`; `ghcr_rebuild_rotation_qualification` 16/16;
  base ubuntu:24.04 digest plus Bazelisk v1.29.0 plus Bazel 9.2.0 plus Cosign
  v2.4.1 plus TUF trust root with manual on-demand rebuild plus rotation,
  sole-maintainer owner, one reviewed PR, customer-flows-only harness plus
  on-demand local docker build with no push/schedule trigger and no extra CI
  job, scheduled CI rebuild rejected; infra only, no Supported claim).
- Devcontainer boot manual plus non-Linux/arm64 wont-fix with fixture evidence qualified seed-only under #648
  (`bazel run //tools/ci:devcontainer_boot_qualification` with
  `tools/ci/tests/fixtures/devcontainer_boot/pins.bzl` plus
  `devcontainer_boot.expected`; `devcontainer_boot_qualification` 16/16;
  seed-host linux/amd64 manual boot verify on every Dockerfile or definition
  change plus before any gated GHCR push with evidence in the same reviewed
  PR, sole-maintainer owner, customer-flows-only harness plus on-demand
  local docker build plus devcontainer up with no boot job in CI and no
  extra CI job, non-Linux plus linux/arm64 boot plus arm64 prebuilt variant
  wont-fix; infra only, no Supported claim).
- Update mutation-event wont-fix plus event-completeness with fixture evidence qualified
  seed-only under #586
  (`bazel run //tools/ci:update_events_qualification` with
  `cli/update/tests/fixtures/update_events/pins.bzl` plus
  `update_events.expected`; `update_events_qualification` 16/16;
  no v1 `change`/`mutation` events and no `changes`/`mutations`/`diagnostics` counts in any
  mode, per-set `notice`/`error` plus `command_finished` is the complete contract with sorted
  order plus `results_complete=true` on live terminal reports, Git scan/BUILD parse/rerun
  rejected, interrupted runs keep preceding per-set events true with no rollback and nothing
  for unattempted sets; protocol-only, no workflow change; platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Atomic update rollback plan with fixture evidence qualified
  seed-only under issue #772
  (`bazel run //tools/ci:update_rollback_qualification` with
  `cli/update/tests/fixtures/update_rollback/pins.bzl` plus
  `update_rollback.expected`; `update_rollback_qualification` 16/16;
  per-set commit boundary never repository-wide with no automatic rollback,
  idempotent retry plus manual `git checkout` restore planned in
  `dx_update::recovery`, `update_recovery` notice plus text hint with no
  silent partial success, interrupted runs keep preceding per-set events true
  with defined retry for unattempted sets; update-only, no resolver change;
  platform plus consumer plus release evidence stays owned gap; no Supported claim).
- Third-party env plugin-model (not planned) plus Go cgo scope with fixture evidence
   qualified seed-only under closed #587 plus issue #789
   (`bazel run //tools/ci:env_plugins_cgo_qualification` with
   `env/tests/fixtures/env_plugins_cgo/pins.bzl` plus
   `env_plugins_cgo.expected`; `env_plugins_cgo_qualification` 24/24;
   no third-party plugin model and no private path (`EnvironmentInfo` repurpose
   wont-fix), pure-Go `GOPACKAGESDRIVER` boundary on rules_go
   0.63.0 with explicit cgo out-of-scope exception and upstream non-guarantee plus
   cgo scope source-only plus strict plus handwritten cgo/race resolved seed-only
   under #789 with `go/tests/fixtures/cgo/` (cgo completion #789), static snapshot
   plus replacement graph plus ambient fallback plus generic parity plus cgo completion claim
   rejected; env only, no PATH-tool collision rule change; platform plus consumer plus release
   evidence stays owned gap under #808; no Supported claim).
- Starlark testing futures with fixture evidence qualified seed-only under closed #588 plus #790 plus #791 plus #792 plus #793 plus #794
  (`bazel run //tools/ci:starlark_futures_qualification` with
  `libs/starlark/tests/fixtures/starlark_futures/pins.bzl` plus
  `starlark_futures.expected` plus `matchers.bzl` plus `aspect_subjects.bzl` plus `toolchain_subjects.bzl` plus `output_group_subjects.bzl` plus `config_subjects.bzl`;
  `starlark_futures_qualification` 37/37;
  per-check filtering plus per-function targets plus Rust orchestration with BEP
  wont-fix on target granularity plus explicit macro instantiation plus single
   invocation with no nested Bazel, richer matchers supported under #790
   (expect_equal plus expect_true/false plus expect_contains plus expect_match
   with greet plus pair-error plus admitted-list plus subject-fields plus
   fingerprint use case via `//libs/starlark/tests:matcher_unit`), aspect
   subjects supported under #791 (DxAspectInfo plus dx_aspect_note plus
   aspect_field observations with leaf plus group use case via
   `//libs/starlark/tests:aspect_subject_analysis`), toolchain subjects
   deferred with the platform plus toolchain mapping plus resolved-report
   use case pinned under #792 (via `toolchain_subjects.bzl` plus
   `//libs/starlark/tests:toolchain_unit`, stays deferred), output-group
   subjects deferred with the group-to-files mapping plus resolved-report
   use case pinned under #794 (via `output_group_subjects.bzl` plus
   `//libs/starlark/tests:output_group_unit`, stays deferred), configuration subjects
   supported under #793 (DxConfigInfo plus config_field observations with
   select plus fragment plus transition use case via
   `//libs/starlark/tests:config_subject_analysis`), action
   including registered-action (#795) subjects deferred pending a concrete use case
   plus fixtures plus successor issue, second Starlark interpreter plus per-check
  `--test_filter` parsing plus nested Bazel plus behavioral matrix as line
  coverage rejected; test framework only, no Bazel semantics change; platform
  plus consumer plus release evidence stays owned gap; no Supported claim).
- CLI execution/reporting gaps with fixture evidence qualified seed-only under #590
  (`bazel run //tools/ci:cli_execution_gaps_qualification` with
  `cli/cli/tests/fixtures/cli_execution_gaps/pins.bzl` plus
  `cli_execution_gaps.expected`; `cli_execution_gaps_qualification` 16/16;
  watch 8 watchable plus 21 not watchable plus CI refusal plus 200ms debounce
  plus verbatim reuse plus one iteration at a time, startup options plus
  test-binary args rejected with `dx bazel` guidance (`dx bazel` unchanged,
  `dx run` to app), report matrix (`lint`/`typecheck`/`check`/`fix` sarif,
  `test` junit, `coverage` lcov, `audit` sarif+spdx, rest none including
  format) with `UnsupportedFormat` never silent substitution, parallelism
  sequential (check/fix phases, watch iterations, update per-set continuation,
  run multirun) with daemon plus parallel umbrella plus inferred forwards plus
  invented reports rejected; CLI-only, no Bazel semantics change; platform
  plus consumer plus release evidence stays owned gap; no Supported claim).
- Layer-4 loss restore-or-wont-fix with fixture evidence qualified seed-only under #645
  (`bazel run //tools/ci:layer4_loss_qualification` with
  `cli/cli/tests/fixtures/layer4_loss/pins.bzl` plus
  `layer4_loss.expected`; `layer4_loss_qualification` 16/16;
  real-daemon exit 3 plus real Buildifier rewrite plus full consumer wiring
  wont-fix (smoke-only for wiring) with hermetic exec code 3 plus
  runner-matrix `x=1` to `x = 1` plus `real_aspect_presence` plus
  `preset_parity_test` plus adopt-rust `dx_dev` smoke staying the customer
  path, nested-Bazel plus non-customer harness plus second Bazel rejected;
  test only, no Bazel semantics change; platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Opt-in strict preset vs default loose with fixture evidence qualified seed-only under #615
  (`bazel run //tools/ci:strict_preset_qualification` with
  `quality/tests/fixtures/strict_preset/pins.bzl` plus loose vs strict
  ruff/biome/tsconfig pairs plus illustrative sources;
  `strict_preset_qualification` 16/16; default stays loose with curated
  plus upstream built-in defaults and no hidden preset, strict is opt-in
  via checked-in native configs with ruff E/F/W/I/N/UP/B/SIM plus Biome
  recommended plus noExplicitAny/useConst/noUnusedVariables plus tsc
  strict true plus Vale markers-only, forcing strict by default plus
  selectable preset IDs plus workspace flags rejected; quality only, no
  default change; platform plus consumer plus release evidence stays owned
  gap; no Supported claim).

The consumer aggregate `dx-ci`
([contract](../github-ci.md#aggregate-status)) is unchanged: stable identity,
disabled-as-skipped stays green, any other non-success fails.
