# Verification matrix (language x layer)

Stage 5 close-out status page:
which verification layer covers which language, with as-built evidence only.
No cell here is a `Supported` claim; promotion to `Supported` requires
release evidence per the [support matrix](../product/support-matrix.md).
Open cells are recorded as gaps; docs describe as-built behavior only.
No cell is `Supported`: missing test/release evidence blocks promotion until
platform plus consumer plus release evidence passes per the support matrix,
enforced by `bazel run //tools/ci:supported_evidence_gate`.
Non-dogfed paths (integration E2E drivers, negative fixtures, the no-coverage cohort,
shell sources with no quality class) stay open under issue #324: they run only under
explicit suites, coverage-excluded runs, or ownership audits, never silently under the
standard dogfood gates.

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
- **Layer-4 E2E**: thin CLI-contract suite in `integration/` (clean,
  dirty, format-roundtrip), explicit invocation only
  (open work).
- **Perf tracking**: Bazel-owned harness with baselines regenerated from
  measured numbers, comparison against the frozen `aspect_rules_lint`
  v2.8.0 baseline as a report (not a gate)
  (open work).
- **Dependency checks**: required-core plus admitted lockfile-consistency and
  declared-dependency usage fixtures delivered in `tools/depcheck/`
  (issues #22, #306); framework-composition depcheck stays with the JS/TS pnpm route.
- **Audit/update live execution**: `dx update` resolver backends per set with independent-set
  continuation and per-set reporting delivered (issue #19); `dx audit` auditor wiring, advisory
  acquisition with 24h cache semantics and offline matching, plus SARIF/SPDX mapping delivered
  (issue #18).
- **Docs pipeline**: per-language adapter runs, link/reference proofs,
  renderer/site artifacts, cache and determinism measurements, guide prose with
  guide-step verification, and first-hour timing proof stay open under issue #310
  (see [Documentation](../documentation/README.md#contracts)); no working site claimed.
  IR plus planning records with fixture evidence qualified seed-only under #310
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

## Status

`Delivered` means implemented and verified on the Linux x86_64 seed host
only (platform qualification open under issue #298). `Open` means open work with no implementation
claimed here. `Planning only` means planning is implemented with live
execution deferred. `Tracked` means measured report-only tracking with no gate.

| Language | Corpus dogfood | Layer-2 matrix | Generation | Examples | E2E | Perf | Depcheck | Audit/update | Docs | Env/codegen |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Delivered | Delivered | Delivered | Delivered (`adopt-rust`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Python | Delivered | Delivered | Delivered | Delivered (`adopt-python`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| JavaScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| TypeScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Go | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-go`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Java | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-java`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Kotlin | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-kotlin`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Scala | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-scala`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| C# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-csharp`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| F# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-fsharp`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| C++ | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-cpp`) | Delivered (contract) | Tracked | Delivered (`tools/depcheck`) | Delivered | Open | Open |
| Vue/Svelte/Astro/MDX | Delivered (code ownership) | Open (regions) | Delivered | Open (composition) | Delivered (contract) | Tracked | Open | Delivered | Open | Open |

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
`.github/workflows/ci.yml` on a clean tree on the Linux x86_64 seed host:

- `build`: `bazel build //...`.
- `test`: `bazel test //...` (manual targets excluded by Bazel tag semantics).
- `coverage`: `bazel run //cli/cli:dx -- coverage --min-coverage 97 //...`
  (seed cell only) plus `bazel run //tools/ci:coverage_report_guards`.
- `prove`: `//tools/ci:manual_negatives`, `:target_tags`, `:coverage_cell`,
  `:coverage_spill`, `:release_hygiene`, `:release_policy`, `:publish_trust`,
  `:shell_contract`.
- `e2e`: `bazel run //tools/ci:e2e_cases` convention guard plus the explicit
  suite `E2E_WORKSPACE=$PWD bazel test //tools/ci:e2e
  --test_env=E2E_WORKSPACE`; manual drivers never run under `//...`.
- `dogfood-freshness`: `bazel run //cli/cli:dx -- generate --check //...`,
  `//tools/ci:corpus_audit`, `:code_ownership`, examples READMEs and
  laziness proofs, quality-cache aquery, depcheck contract, audit/update
  guards, wrapper sources, foundation maps, registry singularity, backlog
  contracts, GHCR hygiene and publish guards, perf/corpus verification,
  widen-update loop, quality/execution/distribution/backlog guards,
  `:supported_evidence_gate`, `:quality_adapters_parity`,
  `:env_codegen_qualification`, `:docs_pipeline_qualification`,
  `:consumer_ci_qualification`, and `:file_family_qualification`.
- `dogfood-lint`, `dogfood-format`, `dogfood-typecheck`: corpus converge then
  `--check` no-op proof, plus lane-A trees `//python/... //javascript/...
  //rust/hello/...` where enforcing.
- `devcontainer-check`, `docs-ci`, `consumer-ci` (build-only self-call).

Green here (static guards on a clean tree, no full rebuild): `e2e_cases`
3/3, `supported_evidence_gate` 20/20, `distribution_closeout_guards` 35/35,
`env_codegen_qualification` 23/23, `docs_pipeline_qualification` 26/26,
`consumer_ci_qualification` 29/29, `file_family_qualification` 24/24.
Full `build`/`test` green is owned by CI on this tree; the last full-tree
record is noted on the issue, not re-claimed here.

Remaining reds stay owned gaps, not green claims:

- Full-tree `dx lint/format/typecheck/test --check //...` over fixtures and
  testdata stays open under #12 (lane A only) and #325 (consumer honesty).
- Per-cell coverage is qualified seed-only under #308 (`tools/coverage/cells.txt`,
  `bazel run //tools/ci:coverage_qualification`; no union, Starlark fallback,
  Codecov opt-in, quotas, local-only remote evidence). First-party PR reporting is
  adopted under #254 (Codecov opt-in only). Non-seed cells stay platform-gated under #298.
- Docs pipeline and environment/codegen stay open under #310 and #309 (see
  [Documentation](../documentation/README.md#contracts)). Environment/codegen
  deferred records plus fixture evidence are qualified seed-only under #309
  (`bazel run //tools/ci:env_codegen_qualification`; no junction/copy
  fallback, no checksum-only fallback, no third-party plugin claim; bootstrap,
  fidelity, spaces, stale-clean, IDE, atomic-commit, BEP, projection,
  root-candidate, and cold/warm tests stay owned gaps). Docs-pipeline IR plus
  planning records with fixture evidence are qualified seed-only under #310
  (`bazel run //tools/ci:docs_pipeline_qualification`; versioned IR, codec,
  planning, frozen contracts, removed stub; adapter runs, renderer/site
  execution, rebuild proof, link completeness, guide-step wiring, timing proof,
  and pin-bump/drift stay owned gaps; no working site claimed).
- Consumer-CI contract plus caller plus gate/aggregate fixture evidence
  qualified seed-only under #312
  (`bazel run //tools/ci:consumer_ci_qualification`; nine checks, explicit
  platforms, fail-closed sequential, stable dx-ci aggregate, hygiene,
  concurrency, permissions, per-cell coverage with fork-safe comments,
  build-only self-call smoke, native bump loop plus Renovate fallback,
  dx migrate planning plus dx run multirun, tag hygiene as-built; platform,
  runner, isolation, cache, ordering, merge, diff, queue, cancellation,
  aggregate binding, thread identity, ordering, limits, fork, untrusted,
  sensitive, retries, Code-Scanning, sequential, tag/release, Renovate and
  native-bot, migrate-execution gaps stay owned gaps); platform qualification
  beyond the seed host stays open under #298.
- File-family quality record with fixture evidence qualified seed-only under #313
  (`bazel run //tools/ci:file_family_qualification`; provider-class
  applicability with never-suffix inference, Starlark/Buildifier plus TOML/Taplo
  adapter-backed evidence, parity-deferred CSS/djlint/buf/yaml/keep-sorted/shell
  plus cue/jsonnet/pkl/qml/terraform routes with owner plus frozen acquisition;
  deferred adapter execution, exact pins/digests/rule-sets/mappings, and platform
  plus consumer plus release evidence stay owned gaps; no Supported claim).
- Non-dogfed paths stay open under #324.

The E2E-case convention lives in the
[integration README](../../integration/README.md#adding-a-case-convention)
(`manual` plus `exclusive`, `local`, `no-sandbox` drivers, `:e2e` suite
membership, explicit-only invocation). The consumer aggregate `dx-ci`
([contract](../github-ci.md#aggregate-status)) is unchanged: stable identity,
disabled-as-skipped stays green, any other non-success fails.
