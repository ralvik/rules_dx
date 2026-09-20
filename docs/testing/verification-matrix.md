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
  guide-step verification, and first-hour timing proof stay open under issue #581
  (live successor to closed #421)
  (see [Documentation](../documentation/README.md#contracts)); no working site claimed.
  IR plus planning records with fixture evidence qualified seed-only under #581
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR schema,
  codec roundtrip/parity/ordering/compat, dx_docs planning units, frozen
  contracts, removed stub behind ADR 0020, with adapter runs, renderer/site
  execution, rebuild proof, link completeness, guide-step wiring, timing proof,
  and pin-bump/drift as owned gaps).
- **Environment/codegen**: deferred/unsupported records plus fixture
  evidence qualified seed-only under #506
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
Rust, JavaScript, TypeScript, Vue, Svelte, Astro, MDX; open tooling work for Python under issue #613).
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
family taxonomy execution with fixture evidence qualified seed-only under #512
(`bazel run //tools/ci:quality_taxonomy_qualification` with
`quality/tests/fixtures/quality_taxonomy/pins.bzl` plus
`quality_taxonomy.expected`; `quality_taxonomy_qualification` 17/17;
taxonomy doc only plus report-not-gate shape only rejected; deferred
adapters plus digests plus platform plus consumer plus release stay owned
gaps; no Supported claim; issue #512 stays taxonomy-only). Python
source-audit tooling with fixture evidence qualified seed-only under #613
(`bazel run //tools/ci:python_audit_qualification` with
`python/tests/fixtures/python_audit/pins.bzl` plus
`python_audit.expected`; `python_audit_qualification` 16/16; curated
audit empty with Bandit excluded plus secrets via Gitleaks, lint
Ruff plus pydoclint plus format Ruff plus typecheck Ty unaffected, no
audit adapter claim, leaving under taxonomy rejected; future selection
plus platform plus consumer plus release stay owned gaps; no Supported
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
  guards, wrapper sources, foundation maps, registry singularity, backlog
  contracts, GHCR hygiene and publish guards,
  widen-update loop, quality/distribution/backlog guards,
  `:supported_evidence_gate`, `:quality_adapters_parity`,
  `:env_codegen_qualification`, `:env_plugins_cgo_qualification` (deferred plugin design plus cgo
  boundary pins plus fixture evidence, issue #587), `:docs_pipeline_qualification`,
  `:consumer_ci_qualification`, `:review_threads_qualification` (frozen 50 plus accounting
  pins plus fixture evidence, issue #592), `:file_family_qualification`,
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
  depcheck pins plus fixture evidence, issue #510),
  `:quality_taxonomy_qualification` (taxonomy execution pins plus
  fixture evidence, issue #512),
  `:python_audit_qualification` (Python source-audit split pins plus
  fixture evidence, issue #613),
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
  `:update_events_qualification` (update mutation wont-fix plus completeness pins plus
  fixture evidence, issue #586),
  `:starlark_futures_qualification` (Starlark filtering plus subjects plus BEP wont-fix/deferred pins plus
  fixture evidence, issue #588),
  `:cli_execution_gaps_qualification` (watch plus forwarding plus reports
  plus parallelism wont-fix pins plus fixture evidence, issue #590),
  `:promotion_checklist_qualification` (promotion checklist pins plus
  fixture evidence, issue #611),
  `:sbom_upload_qualification` (SBOM plus provenance CI upload pins plus
  fixture evidence, issue #612),
  `:musl_qualification`, `:macos_qualification` (arm64 plus x86_64
  best-effort), `:windows_qualification`, `:ci_matrix_qualification`
  (host matrix, issue #415), `:flakiness_qualification`
  (flaky retries plus tuned timeouts plus sharding, issue #619), and `:closeout_battery_qualification`
  (battery commands plus docs gate, issue #467).
- `devcontainer-check`, `docs-ci`, `dogfood (test-disabled self-call on
  linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus
  windows_x86_64, issue
  #408 plus Phase 1 #607 coverage superset, verbatim `//...`).

Green here (static guards on a clean tree, no full rebuild):
`non_dogfed_paths`, `non_dogfed_qualification` 16/16, `supported_evidence_gate`, `distribution_closeout_guards`,
`env_codegen_qualification` 32/32, `env_plugins_cgo_qualification` 16/16, `docs_pipeline_qualification` 33/33,
`consumer_ci_qualification` 43/43, `review_threads_qualification` 16/16, `file_family_qualification` 24/24,
`helper_qualification` 27/27, `clap_tokenizer_qualification` 19/19,
`hello_smoke_qualification` 16/16, `parser_sample_qualification` 23/23, `rustfmt_edition_qualification` 20/20, `cc_optout_qualification` 18/18, `shell_env_qualification` 15/15, `bindgen_qualification` 16/16, `cxx_identity_qualification` 18/18, `exact_target_qualification` 16/16, `junit_qualification` 16/16, `xunit_qualification` 16/16, `gotest_qualification` 16/16, `googletest_qualification` 16/16, `scalatest_qualification` 16/16, `paket_qualification` 16/16, `godeps_qualification` 16/16, `cc_hermetic_qualification` 16/16, `jvm_quality_qualification` 16/16, `scala_dotnet_defaults_qualification` 17/17, `native_quality_qualification` 17/17, `structured_defaults_qualification` 17/17, `file_family_defaults_qualification` 17/17, `scalafix_qualification` 12/12, `roslyn_qualification` 13/13, `fsharplint_qualification` 13/13, `stable_stack_qualification` 16/16, `musl_qualification` 12/12, `macos_qualification` 12/12,
<<<<`windows_qualification` 13/13, `windows_acquisition_qualification` 14/14, `acquisition_rights_qualification` 15/15, `windows_transport_qualification` 16/16, `prebuilt_interop_qualification` 16/16, `linux_corpus_qualification` 16/16, `lcov_accounting_qualification` 16/16, `cargo_metadata_qualification` 16/16, `strict_generation_qualification` 16/16, `cross_routes_qualification` 17/17, `remediation_bounds_qualification` 16/16, `layer2_opens_qualification` 16/16, `quality_taxonomy_qualification` 17/17, `python_audit_qualification` 16/16, `selective_update_qualification` 16/16, `selective_cargo_qualification` 16/16, `selective_nuget_qualification` 16/16, `selective_maven_qualification` 16/16, `selective_go_qualification` 16/16, `bump_chain_qualification` 16/16, `update_events_qualification` 16/16, `env_plugins_cgo_qualification` 16/16, `starlark_futures_qualification` 16/16, `cli_execution_gaps_qualification` 16/16, `promotion_checklist_qualification` 16/16, `sbom_upload_qualification` 17/17, `ci_matrix_qualification` 14/14, `flakiness_qualification` 16/16, `closeout_battery_qualification` 24/24, `coverage_qualification` 33/33.
Full `build`/`test` green is owned by CI on this tree via `bazel run //tools/ci:closeout_battery_qualification` (issue #467;
battery commands plus docs gate pinned, full rebuild owned by CI jobs, not re-claimed here).
Full-tree `dx lint/format/typecheck/test --check //...` over fixtures and
testdata is green via the dogfood test-disabled self-call (issue #408 plus
Phase 1 #607 coverage superset, verbatim `//...` in Battery above) with fixtures/testdata expected failures
owned by #508 (`bazel run //tools/ci:non_dogfed_qualification` with
`tools/ci/tests/fixtures/non_dogfed/pins.bzl` plus `non_dogfed.expected`;
negatives per issue #406, hermetic CLI-contract per issue #407; closed #12
lane A and closed #325 carry no open scope).

Remaining reds stay owned gaps, not green claims:
- Per-cell coverage is qualified for the seed plus arm64 plus two static-musl plus macos arm64 plus macos x86_64 best-effort plus windows x86_64 cells with fixture evidence under
  #507 (`tools/coverage/tests/fixtures/per_cell/pins.bzl` plus
  `per_cell.expected` plus `codecov_remote.expected` via
  `bazel run //tools/ci:coverage_qualification`; seven cells, same
  scope, no union, Starlark fallback, Codecov opt-in, quotas, local-only
  remote evidence). First-party PR reporting is
  adopted under #254 (Codecov opt-in only; the seed cell owns the PR comment,
  the arm64 plus musl plus macos plus macos-x86_64 plus windows cells report to their job summaries; the macos x86_64 best-effort cell reports without blocking required-host release). All required plus best-effort cells are qualified; out-of-v1 hosts stay platform-gated under #298.
- Docs pipeline and environment/codegen stay open under #581 and #506 (see
  [Documentation](../documentation/README.md#contracts)). Environment/codegen
  deferred records plus fixture evidence are qualified seed-only under #506
  (`bazel run //tools/ci:env_codegen_qualification` with
  `env/tests/fixtures/env_codegen/pins.bzl` plus `env_codegen.expected`
  plus `roots_bep.txt`; no junction/copy fallback, no checksum-only
  fallback, no third-party plugin claim; bootstrap, fidelity, spaces,
  stale-clean, IDE, atomic-commit, BEP, projection, root-candidate, and
  cold-warm qualified with WP shard plus root plus collector evidence;
  platform plus consumer plus release evidence stays owned gap; no Supported
  claim). Docs-pipeline IR plus
  planning records with fixture evidence are qualified seed-only under #581
  (live successor to closed #421)
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR, codec,
  planning, frozen contracts, removed stub; adapter runs, renderer/site
  execution, rebuild proof, link completeness, guide-step wiring, timing proof,
  and pin-bump/drift stay owned gaps; no working site claimed).
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
  fixture pair with no adapter claim; digests plus adapters stay owned under #420;
  `protobuf`/`qml` stay owned by issue #419, never double-claimed
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
  with no adapter claim; digests plus `scala` adapter stays owned under #417
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
  with no adapter claim; digests plus `csharp` adapter stays owned under #417
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
  adapter claim; digests plus `fsharp` adapter stays owned under #417
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
  qualified seed-only under issue #510
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
  `layer2_opens.expected`; Closed #303 only, #416-420 adapters partially
  with digests plus adapters staying owned, adapter-less as pass rejected;
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
- Quality family taxonomy execution with fixture evidence qualified seed-only under #512
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
   416-420 plus 307, digests plus rule-sets owned by cohorts, platform plus
   consumer plus release evidence stays owned gap; backends provisional; no
   Supported claim; issue #512 stays taxonomy-only).
- Python source-audit split with fixture evidence qualified seed-only
   under #613
   (`bazel run //tools/ci:python_audit_qualification` with
   `python/tests/fixtures/python_audit/pins.bzl` plus
   `python_audit.expected`; `python_audit_qualification` 16/16;
   curated audit empty with Bandit excluded plus secrets via Gitleaks,
   lint Ruff plus pydoclint plus format Ruff plus typecheck Ty
   unaffected with flake8 plus pylint opt-ins, no audit adapter claim,
   source audit distinct from ecosystem audit/update delivered, leaving
   under taxonomy rejected with mismatched scope; future selection plus
   platform plus consumer plus release evidence stays owned gap;
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
- Third-party env plugin-model design plus Go cgo exception boundary with fixture evidence
  qualified seed-only under #587
  (`bazel run //tools/ci:env_plugins_cgo_qualification` with
  `env/tests/fixtures/env_plugins_cgo/pins.bzl` plus
  `env_plugins_cgo.expected`; `env_plugins_cgo_qualification` 16/16;
  deferred third-party plugin model with design owner plus acceptance criteria and no private
  path, `EnvironmentInfo` repurpose wont-fix, pure-Go `GOPACKAGESDRIVER` boundary on rules_go
  0.63.0 with explicit cgo out-of-scope exception and upstream non-guarantee, static snapshot
  plus replacement graph plus ambient fallback plus generic parity plus cgo completion claim
  rejected; env only, no PATH-tool collision rule change; platform plus consumer plus release
  evidence stays owned gap; no Supported claim).
- Starlark testing futures with fixture evidence qualified seed-only under #588
  (`bazel run //tools/ci:starlark_futures_qualification` with
  `libs/starlark/tests/fixtures/starlark_futures/pins.bzl` plus
  `starlark_futures.expected`; `starlark_futures_qualification` 16/16;
  per-check filtering plus per-function targets plus Rust orchestration with BEP
  wont-fix on target granularity plus explicit macro instantiation plus single
  invocation with no nested Bazel, richer matchers plus aspect plus toolchain
  plus configuration (including transitions) plus output-group plus action
  (including registered-action) subjects deferred pending a concrete use case
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
