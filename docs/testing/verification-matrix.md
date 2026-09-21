# Verification matrix (language x layer)

Close-out status page:
which verification layer covers which language, with as-built evidence only.
No cell here is a `Supported` claim; promotion to `Supported` requires
release evidence per the [support matrix](../product/support-matrix.md).
Open cells are recorded as gaps; docs describe as-built behavior only.
No cell is `Supported`: missing test/release evidence blocks promotion until
platform plus consumer plus release evidence passes per the support matrix,
enforced by `bazel run //tools/ci:supported_evidence_gate`.
Non-dogfed execution plan qualified seed-only under #508
(`bazel run //tools/ci:non_dogfed_qualification`; e2e plus negatives plus
no-coverage plus shell with fixture evidence pinned in
`tools/ci/tests/fixtures/non_dogfed/pins.bzl` plus `non_dogfed.expected`;
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
  generation via the customer check flow under issue #644 (successor to closed #503):
  `dogfood-freshness` `generate --check //...` plus consumer-CI `generate --check`
  plus bump-PR `generate --check` verify; custom freshness harness rejected.
- **Examples external-consumer**: per-foundation `adopt-*` workspaces
  proving generation as a consumer, plus acquisition/laziness proof
   (delivered on the seed host; platform/remote dimensions owned by
   #803-#807, successors to closed #298).
- **Layer-4 CLI-contract (issue #407, hermetic, replaces nested E2E)**:
  `dx test`/`dx build` exit-code preservation via `cli/cli/src/exec` unit
  pins (including Bazel test-failure code 3), format rewrite `x=1` →
  `x = 1` via `//quality/testdata:runner_matrix` goldens, aspect wiring via
  `DxSubjectInfo` pins (`//quality/testdata:real_aspect_presence`), preset
  check→update→recheck via `//:preset_parity_test`; `local_path_override` +
  `dx_dev` wiring via the adopt-rust smoke in normal CI
  (`bazel build //examples/adopt-rust/... --config=dx_dev` in the build job).
  What is lost: real-daemon exit 3, real Buildifier rewrite, full consumer
  wiring now smoke-only. Per-loss dispositions qualified seed-only under #645
  (`bazel run //tools/ci:layer4_loss_qualification` with
  `cli/cli/tests/fixtures/layer4_loss/pins.bzl` plus `layer4_loss.expected`;
  all three wont-fix with hermetic pins staying the customer path and the
  nested-Bazel plus non-customer harness rejected; no Supported claim).
- **Dependency checks**: required-core plus admitted lockfile-consistency and
  declared-dependency usage fixtures delivered in `tools/depcheck/`
   (issue #22; remaining opens under #796-#800, successors to closed
   #510); framework-composition depcheck stays with the JS/TS pnpm route.
- **Audit/update live execution**: `dx update` resolver backends per set with independent-set
  continuation and per-set reporting delivered (issue #19); `dx audit` auditor wiring, advisory
  acquisition with 24h cache semantics and offline matching, plus SARIF/SPDX mapping delivered
  (issue #18). `Audit/update` here records repo-wide ecosystem live execution, distinct from
  per-language source audit in the [support matrix](../product/support-matrix.md#application-foundations).
- **Docs pipeline**: per-language adapter runs, link/reference proofs,
   renderer/site artifacts, cache and determinism measurements, guide prose with
   guide-step verification, and first-hour timing proof stay open under #779-#785
   (successors to closed #581, live successor to closed #421)
   (see [Documentation](../documentation/README.md#contracts)); no working site claimed.
   IR plus planning records with fixture evidence qualified seed-only under closed #581
   (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR schema,
   codec roundtrip/parity/ordering/compat, dx_docs planning units, frozen
   contracts, removed stub behind ADR 0020, with adapter runs, renderer/site
   execution, rebuild proof, link completeness, guide-step wiring, timing proof,
   and pin-bump/drift as owned gaps under #779-#785). That qualification is
   planning-only green, not pipeline-green.
- **Environment/codegen**: deferred/unsupported records plus fixture
   evidence qualified seed-only under closed #506
  (`bazel run //tools/ci:env_codegen_qualification`; public protocol,
  Windows fallback, standalone, signing/trust, plus bootstrap/fidelity/
  stale/IDE/atomic-commit/BEP/projection/roots/cold-warm with WP1-WP5 plus
  `env/tests/fixtures/env_codegen/pins.bzl` plus `env_codegen.expected`
  plus `roots_bep.txt`; platform plus consumer plus release evidence stays
  owned gap; no Supported claim).
- **Non-dogfed execution plan**: the cohorts that never run under the
  standard dogfood gates each have an explicit path, qualified seed-only
  under #508 (`bazel run //tools/ci:non_dogfed_qualification` with
  `tools/ci/tests/fixtures/non_dogfed/pins.bzl` plus `non_dogfed.expected`;
  `non_dogfed_qualification` 16/16; hermetic pins under issue #407) and pinned by
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
  `shell_contract`; elimination wont-fix under issue #667).

## Status

`Delivered` means implemented and verified on the Linux x86_64 seed host
plus Linux arm64 native (issue #410) plus the two Linux static-musl
profiles (issue #411) plus macOS arm64 native (issue #412) plus macOS
x86_64 best-effort native (issue #413) plus Windows x86_64 MSVC-compatible
 native (issue #414). `Delivered` here is verification-layer evidence only, not
support-matrix promotion: it never promotes a support-matrix `Planned` cell to
`Seed-host-delivered`, `Platform-qualified`, or `Supported`. `Open` means open work under its owning tracker with no
implementation claimed here. Owning trackers for table `Open` cells: Docs under #779-#785 (successors to closed #581,
live successor to closed #421); Env/codegen under #787 plus onboarding #788 (successors to closed #506); Layer-2
adapter-less plus regions plus framework-composition Depcheck under #796-#800 (successors to closed #416-#420, delivery
qualified seed-only under closed #510) with taxonomy promotion under #802 (successor to closed #512) and Python future
selection under #801 (successor to closed #613). `Planning only` means planning is implemented with live
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
Rust, JavaScript, TypeScript, Vue, Svelte, Astro, MDX; Python source-audit tooling qualified
seed-only under closed #613 with future selection open under #801).
`Support-matrix Planned` cells claim accepted scope only; where this matrix shows `Open`,
the corresponding `Planned` cell is scope with open implementation, and where this
matrix shows `Delivered`, the corresponding `Planned` cell is scope with seed-host
layer evidence, still not promoted under the support-matrix lifecycle.

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
 family taxonomy execution with fixture evidence qualified seed-only under closed #512
(`bazel run //tools/ci:quality_taxonomy_qualification` with
`quality/tests/fixtures/quality_taxonomy/pins.bzl` plus
`quality_taxonomy.expected`; `quality_taxonomy_qualification` 17/17;
   taxonomy doc only plus report-not-gate shape only rejected; deferred
   adapters plus digests plus platform plus consumer plus release stay owned
   gaps under #802 (successor to closed #512); no Supported claim; closed #512 stays taxonomy-only). Python
   source-audit tooling with fixture evidence qualified seed-only under closed #613
(`bazel run //tools/ci:python_audit_qualification` with
`python/tests/fixtures/python_audit/pins.bzl` plus
`python_audit.expected`; `python_audit_qualification` 16/16; curated
audit empty with Bandit excluded plus secrets via Gitleaks, lint
Ruff plus pydoclint plus format Ruff plus typecheck Ty unaffected, no
   audit adapter claim, leaving under taxonomy rejected; future selection (#801) plus
   platform plus consumer plus release (#808) stay owned gaps; no Supported
   claim).

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

CI flakiness plus timeout tuning qualified seed-only under #619
(`bazel run //tools/ci:flakiness_qualification`; every direct `bazel test`
carries `--flaky_test_attempts=3 --test_timeout=300`, seed test/coverage at 45
minutes plus per-host test/coverage at 60 minutes with no blanket 90, sharding
stays per-host/per-stage with ordinary Bazel intra-job sharding and no
`strategy.matrix`, long-timeouts-only rejected; `flakiness_qualification` 16/16;
CI only, no Supported claim).

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
- `test`: `bazel test --flaky_test_attempts=3 --test_timeout=300 //...` (no manual tests; hermetic CLI-contract pins run here; seed 45m plus per-host 60m, issue #619).
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
  `:non_dogfed_qualification` (e2e plus negatives pins plus fixture evidence, issue #508),
  examples READMEs and
  laziness proofs, quality-cache aquery, depcheck contract, audit/update
  guards, product runtime guards (`:product_runtime_guards`), wrapper sources, foundation maps, registry singularity, backlog
  contracts, GHCR hygiene and publish guards,
  widen-update loop, quality/distribution/backlog guards,
  `:supported_evidence_gate`, `:quality_adapters_parity`,
   `:env_codegen_qualification`, `:env_plugins_cgo_qualification` (cgo
   boundary pins plus fixture evidence, closed #587; plugin model not planned,
   cgo completion #789), `:docs_pipeline_qualification`,
  `:consumer_ci_qualification`, `:review_threads_qualification` (frozen 50 plus accounting
  pins plus fixture evidence, issue #592), `:file_family_qualification`,
  `:helper_qualification`, `:clap_tokenizer_qualification`,
  `:hello_smoke_qualification`, `:parser_sample_qualification`,
  `:rustfmt_edition_qualification`, `:cc_optout_qualification`,
  `:shell_env_qualification`, `:bindgen_qualification`,
  `:cxx_identity_qualification`, `:exact_target_qualification`, `:cpp_snapshot_qualification`, `:junit_qualification`,
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
  `:file_family_defaults_qualification` (file-family versions plus rule-sets
  pins plus fixture evidence, issue #489),
  `:scalafix_qualification` (Scalafix console plus semanticdb-classpath wiring
  pins plus fixture evidence, issue #490),
  `:roslyn_qualification` (Roslyn per-TFM-RID SARIF aggregation pins plus
  fixture evidence, issue #492),
  `:fsharplint_qualification` (FSharpLint console vs library-API binding
  pins plus fixture evidence, issue #493),
  `:stable_stack_qualification` (stable-stack compose pins plus
  fixture evidence, issue #494),
  `:windows_acquisition_qualification` (Windows immutable lazy fetch pins
  plus fixture evidence, issue #495),
  `:acquisition_rights_qualification` (Apple plus Microsoft rights pins
  plus fixture evidence, issue #496),
  `:windows_transport_qualification` (Windows transport plus ABI pins
  plus fixture evidence, issue #497),
  `:prebuilt_interop_qualification` (prebuilt interop pins
  plus fixture evidence, issue #498),
  `:linux_corpus_qualification` (linux corpus pins
  plus fixture evidence, issue #499),
  `:lcov_accounting_qualification` (LCOV accounting pins
  plus fixture evidence, issue #501),
  `:cargo_metadata_qualification` (Cargo metadata pins
  plus fixture evidence, issue #502),
  `:strict_generation_qualification` (strict generation pins
  plus fixture evidence, issue #503),
  `:cross_routes_qualification` (cross routes pins
  plus fixture evidence, issue #504),
  `:remediation_bounds_qualification` (bounded remediation pins
  plus fixture evidence, issue #505),
   `:layer2_opens_qualification` (Layer-2 adapter-less plus composition plus
   depcheck pins plus fixture evidence, closed #510; delivery now #754, #796-#800),
   `:quality_taxonomy_qualification` (taxonomy execution pins plus
   fixture evidence, closed #512; promotion gaps #802),
   `:python_audit_qualification` (Python source-audit split pins plus
   fixture evidence, closed #613; future selection #801),
  `:selective_update_qualification` (per-set selective vs wont-fix pins plus
  fixture evidence, issue #583),
  `:selective_cargo_qualification` (Cargo per-crate wont-fix pins plus
  fixture evidence, issue #633),
  `:selective_nuget_qualification` (NuGet per-package wont-fix pins plus
  fixture evidence, issue #635),
  `:selective_maven_qualification` (Maven per-artifact wont-fix pins plus
  fixture evidence, issue #634),
  `:selective_go_qualification` (Go per-module wont-fix plus full no-op pins plus
  fixture evidence, issue #636),
  `:bump_chain_qualification` (bump widen plus automatic refresh pins plus
  fixture evidence, issue #638),
  `:bump_discovery_qualification` (bump outdated discovery pins plus
  fixture evidence, issue #639),
  `:bump_gha_qualification` (bump GHA tag-to-SHA auto pins plus
  fixture evidence, issue #640),
  `:runner_rotation_qualification` (runner plus SDK rotation cadence plus
  retirement pins plus fixture evidence, issue #642),
  `:ghcr_rebuild_rotation_qualification` (GHCR rebuild plus signing rotation
  manual pins plus fixture evidence, issue #647),
  `:devcontainer_boot_qualification` (devcontainer boot manual plus
  non-Linux/arm64 wont-fix pins plus fixture evidence, issue #648),
  `:update_events_qualification` (update mutation wont-fix plus completeness pins plus
  fixture evidence, issue #586),
   `:starlark_futures_qualification` (Starlark filtering plus subjects plus BEP wont-fix/deferred pins plus
   fixture evidence, closed #588; successors #790-#795),
  `:cli_execution_gaps_qualification` (watch plus forwarding plus reports
  plus parallelism wont-fix pins plus fixture evidence, issue #590),
  `:layer4_loss_qualification` (real-daemon plus Buildifier plus wiring
  wont-fix pins plus fixture evidence, issue #645),
   `:promotion_checklist_qualification` (promotion checklist pins plus
   fixture evidence, closed #611; process now #808),
   `:sbom_upload_qualification` (SBOM plus provenance CI upload pins plus
   fixture evidence, closed #612; per-host release evidence #803-#807),
   `:musl_qualification`, `:macos_qualification` (arm64 plus x86_64
   best-effort), `:windows_qualification`, `:ci_matrix_qualification`
   (host matrix, closed #415), `:flakiness_qualification`
  (flaky retries plus tuned timeouts plus sharding, issue #619), and `:closeout_battery_qualification`
  (battery commands plus docs gate, issue #467).
- `devcontainer-check`, `docs-ci`, `dogfood (test-disabled self-call on
  linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus
  windows_x86_64, issue
  #408 plus Phase 1 #607 coverage superset, verbatim `//...`).

Green here (static guards on a clean tree, no full rebuild):
`non_dogfed_paths`, `non_dogfed_qualification` 16/16, `supported_evidence_gate`, `distribution_closeout_guards`, `product_runtime_guards` 20/20,
`env_codegen_qualification` 32/32, `env_plugins_cgo_qualification` 16/16, `docs_pipeline_qualification` 33/33 (planning-only green: IR plus planning records only; adapter runs, renderer/site execution, rebuild proof, link completeness, guide-step wiring, timing proof, and pin-bump/drift stay open under #779-#785 with no working site claimed),
`consumer_ci_qualification` 43/43, `review_threads_qualification` 16/16, `file_family_qualification` 24/24,
`helper_qualification` 27/27, `clap_tokenizer_qualification` 19/19,
`hello_smoke_qualification` 16/16, `parser_sample_qualification` 23/23, `rustfmt_edition_qualification` 20/20, `cc_optout_qualification` 18/18, `shell_env_qualification` 15/15, `bindgen_qualification` 16/16, `cxx_identity_qualification` 18/18, `exact_target_qualification` 16/16, `cpp_snapshot_qualification` 17/17, `junit_qualification` 16/16, `xunit_qualification` 16/16, `gotest_qualification` 16/16, `googletest_qualification` 16/16, `scalatest_qualification` 16/16, `paket_qualification` 16/16, `godeps_qualification` 16/16, `cc_hermetic_qualification` 16/16, `jvm_quality_qualification` 16/16, `scala_dotnet_defaults_qualification` 17/17, `native_quality_qualification` 17/17, `structured_defaults_qualification` 17/17, `file_family_defaults_qualification` 17/17, `scalafix_qualification` 12/12, `roslyn_qualification` 13/13, `fsharplint_qualification` 13/13, `stable_stack_qualification` 16/16, `musl_qualification` 12/12, `macos_qualification` 12/12,
<<<<`windows_qualification` 13/13, `windows_acquisition_qualification` 14/14, `acquisition_rights_qualification` 15/15, `windows_transport_qualification` 16/16, `prebuilt_interop_qualification` 16/16, `linux_corpus_qualification` 16/16, `lcov_accounting_qualification` 16/16, `cargo_metadata_qualification` 16/16, `strict_generation_qualification` 16/16, `cross_routes_qualification` 17/17, `remediation_bounds_qualification` 16/16, `layer2_opens_qualification` 16/16, `quality_taxonomy_qualification` 17/17, `python_audit_qualification` 16/16, `selective_update_qualification` 16/16, `selective_cargo_qualification` 16/16, `selective_nuget_qualification` 16/16, `selective_maven_qualification` 16/16, `selective_go_qualification` 16/16, `bump_chain_qualification` 16/16, `bump_discovery_qualification` 16/16, `bump_gha_qualification` 16/16, `runner_rotation_qualification` 16/16, `ghcr_rebuild_rotation_qualification` 16/16, `devcontainer_boot_qualification` 16/16, `update_events_qualification` 16/16, `env_plugins_cgo_qualification` 16/16, `starlark_futures_qualification` 16/16, `cli_execution_gaps_qualification` 16/16, `layer4_loss_qualification` 16/16, `promotion_checklist_qualification` 16/16, `sbom_upload_qualification` 17/17, `ci_matrix_qualification` 14/14, `flakiness_qualification` 16/16, `closeout_battery_qualification` 24/24, `coverage_qualification` 33/33.
Full `build`/`test` green is owned by CI on this tree via `bazel run //tools/ci:closeout_battery_qualification` (issue #467;
battery commands plus docs gate pinned, full rebuild owned by CI jobs, not re-claimed here).
Full-tree `dx lint/format/typecheck/test --check //...` over fixtures and
testdata is green via the dogfood test-disabled self-call (issue #408 plus
Phase 1 #607 coverage superset, verbatim `//...` in Battery above) with fixtures/testdata expected failures
owned by #508 (`bazel run //tools/ci:non_dogfed_qualification` with
`tools/ci/tests/fixtures/non_dogfed/pins.bzl` plus `non_dogfed.expected`;
negatives per issue #406, hermetic CLI-contract per issue #407; closed #12
lane A and closed #325 carry no open scope).

Remaining reds moved to [`verification-matrix-remaining.md`](./verification-matrix-remaining.md) (split, no content change).
