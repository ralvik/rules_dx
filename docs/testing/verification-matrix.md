# Verification matrix (language x layer)

Stage 5 close-out status page:
which verification layer covers which language, with as-built evidence only.
No cell here is a `Supported` claim; promotion to `Supported` requires
release evidence per the [support matrix](../product/support-matrix.md).
Open cells are recorded as gaps; docs describe as-built behavior only.
No cell is `Supported`: missing test/release evidence blocks promotion until
platform plus consumer plus release evidence passes per the support matrix,
enforced by `bazel run //tools/ci:supported_evidence_gate`.
Non-dogfed execution plan delivered under issues #324/#407
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
  generation (open work).
- **Examples external-consumer**: per-foundation `adopt-*` workspaces
  proving generation as a consumer, plus acquisition/laziness proof
  (delivered on the seed host; platform/remote dimensions owned by
  #298/#308).
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
  (issues #22, #306); framework-composition depcheck stays with the JS/TS pnpm route.
- **Audit/update live execution**: `dx update` resolver backends per set with independent-set
  continuation and per-set reporting delivered (issue #19); `dx audit` auditor wiring, advisory
  acquisition with 24h cache semantics and offline matching, plus SARIF/SPDX mapping delivered
  (issue #18).
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
  evidence qualified seed-only under #309
  (`bazel run //tools/ci:env_codegen_qualification`; public protocol,
  Windows fallback, standalone, signing/trust, plus bootstrap/lock/roots/
  collector/env-plan/node projection evidence with unproven tests as owned
  gaps).
- **Non-dogfed execution plan**: the cohorts that never run under the
  standard dogfood gates each have an explicit path, pinned by
  `bazel run //tools/ci:non_dogfed_paths` (issues #324/#407): CLI-contract
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
native (issue #414). `Open` means open work with no implementation
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
open work.

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
  `:musl_qualification`, `:macos_qualification` (arm64 plus x86_64
  best-effort), `:windows_qualification`, and `:ci_matrix_qualification`
  (host matrix, issue #415).
- `devcontainer-check`, `docs-ci`, `consumer-ci` (all-enabled self-call on
  linux_x86_64 plus linux_arm64 plus macos_arm64 plus macos_x86_64 plus
  windows_x86_64, issue
  #408, verbatim `//...`).

Green here (static guards on a clean tree, no full rebuild):
`non_dogfed_paths`, `supported_evidence_gate`, `distribution_closeout_guards`,
`env_codegen_qualification` 23/23, `docs_pipeline_qualification` 33/33,
`consumer_ci_qualification` 30/30, `file_family_qualification` 24/24,
`helper_qualification` 27/27, `clap_tokenizer_qualification` 19/19,
`musl_qualification` 12/12, `macos_qualification` 12/12,
`windows_qualification` 13/13, `ci_matrix_qualification` 14/14.
Full `build`/`test` green is owned by CI on this tree; the last full-tree
record is noted on the issue, not re-claimed here.

Remaining reds stay owned gaps, not green claims:

- Full-tree `dx lint/format/typecheck/test --check //...` over fixtures and
  testdata stays open under #12 (lane A only) and #325 (consumer honesty).
- Per-cell coverage is qualified for the seed plus arm64 plus two static-musl plus macos arm64 plus macos x86_64 best-effort plus windows x86_64 cells under
  #308/#410/#411/#412/#413/#414 (`tools/coverage/cells.txt`,
  `bazel run //tools/ci:coverage_qualification`; no union, Starlark fallback,
  Codecov opt-in, quotas, local-only remote evidence). First-party PR reporting is
  adopted under #254 (Codecov opt-in only; the seed cell owns the PR comment,
  the arm64 plus musl plus macos plus macos-x86_64 plus windows cells report to their job summaries; the macos x86_64 best-effort cell reports without blocking required-host release). All required plus best-effort cells are qualified; out-of-v1 hosts stay platform-gated under #298.
- Docs pipeline and environment/codegen stay open under #421 and #309 (see
  [Documentation](../documentation/README.md#contracts)). Environment/codegen
  deferred records plus fixture evidence are qualified seed-only under #309
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
  qualified seed-only under #312
  (`bazel run //tools/ci:consumer_ci_qualification`; nine checks, explicit
  platforms, fail-closed sequential, stable dx-ci aggregate, hygiene,
  concurrency, permissions, per-cell coverage with fork-safe comments,
  all-enabled self-call (issue #408, verbatim `//...`), native bump loop plus Renovate (complementary,
  issue #326),
   dx migrate planning plus dx run multirun, tag hygiene as-built; platform,
   runner, isolation, cache, ordering, merge, diff, queue, cancellation,
   aggregate binding, thread identity, ordering, limits, fork, untrusted,
   sensitive, retries, Code-Scanning, sequential, tag/release, Renovate and
   native-bot, migrate-execution gaps stay owned gaps); platform qualification
   beyond the seed plus arm64 plus musl plus macos plus macos-x86_64 plus
   windows hosts stays open under #298 (arm64 qualified under #410, static
   musl under #411, macos arm64 under #412, macos x86_64 best-effort under
   #413, windows x86_64 under #414).
- File-family quality record with fixture evidence qualified seed-only under #313
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
- Non-dogfed execution plan delivered under #324/#407
  (`bazel run //tools/ci:non_dogfed_paths`; hermetic CLI-contract pins,
  green hermetic failure proofs (issue #406), coverage-excluded runs, shell ownership
  plus test execution with no quality class by design).

The consumer aggregate `dx-ci`
([contract](../github-ci.md#aggregate-status)) is unchanged: stable identity,
disabled-as-skipped stays green, any other non-success fails.
