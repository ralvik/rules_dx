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

- **Corpus dogfood**: `real_source_target(name = "corpus")` per package
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
- **Dependency checks**: per-language lockfile-consistency and
  declared-dependency usage fixtures (contract accepted, fixtures open)
  (open work).
- **Audit/update live execution**: auditor wiring, advisory acquisition,
  SARIF/SPDX mapping, resolver backends, per-set reporting
  (open work,
  open work).
- **Docs pipeline**: per-language adapter runs, link/reference proofs,
  renderer/site artifacts, guide-step verification
  (open work).
- **Environment/codegen**: projection queries, replacement contracts,
  boundary evidence (open work).

## Status

`Delivered` means implemented and verified on the Linux x86_64 seed host
only (platform qualification open under issue #298). `Open` means open work with no implementation
claimed here. `Planning only` means planning is implemented with live
execution deferred. `Tracked` means measured report-only tracking with no gate.

| Language | Corpus dogfood | Layer-2 matrix | Generation | Examples | E2E | Perf | Depcheck | Audit/update | Docs | Env/codegen |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Delivered | Delivered | Delivered | Delivered (`adopt-rust`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Python | Delivered | Delivered | Delivered | Delivered (`adopt-python`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| JavaScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| TypeScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Go | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-go`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Java | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-java`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Kotlin | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-kotlin`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Scala | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-scala`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| C# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-csharp`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| F# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-fsharp`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| C++ | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-cpp`) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |
| Vue/Svelte/Astro/MDX | Delivered (code ownership) | Open (regions) | Delivered | Open (composition) | Delivered (contract) | Tracked | Open | Planning only | Open | Open |

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

The full close-out battery (open work)
runs on a clean tree after the staged issues land:

```sh
bazel build //...
bazel test //...
bazel run //cli/cli:dx -- generate --check //...
```

plus the corpus and code ownership audits, the per-harness audits in
`dogfood-freshness`, and the explicit E2E suite. Remaining reds each name
their open gap above; docs match as-built behavior.
