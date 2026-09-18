# Verification matrix (language x layer)

Stage 5 close-out status page ([issue #54](https://github.com/ralvik/rules_dx/issues/54)):
which verification layer covers which language, with as-built evidence only.
No cell here is a `Supported` claim; promotion to `Supported` requires
release evidence per the [support matrix](../product/support-matrix.md).
Open cells link to their owning issue; docs describe as-built behavior and
link to the owning issue for open work.

## Layers

- **Corpus dogfood**: `real_source_target(name = "corpus")` per package
  checked with the real lint/format aspects at `--fail_on warning`, plus
  the corpus and code ownership audits. Scope: Markdown/Starlark/TOML
  target-less files (corpus) and production code on normal targets
  ([issue #12](https://github.com/ralvik/rules_dx/issues/12) lane A).
- **Layer-2 runner matrix**: real `quality_runner` over provider-less
  data, one case per supported language x capability cell x {pass, fail}
  ([quality/runner-matrix](../quality/runner-matrix.md)).
- **Generation freshness**: `dx generate --check //...` must be a
  deterministic no-op on a clean checkout; BUILD/corpus sync is owned by
  generation ([issue #15](https://github.com/ralvik/rules_dx/issues/15)).
- **Examples external-consumer**: per-foundation `adopt-*` workspaces
  proving generation as a consumer, plus acquisition/laziness proof
  ([issue #85](https://github.com/ralvik/rules_dx/issues/85)).
- **Layer-4 E2E**: thin CLI-contract suite in `integration/` (clean,
  dirty, format-roundtrip), explicit invocation only
  ([issue #55](https://github.com/ralvik/rules_dx/issues/55)).
- **Perf tracking**: Bazel-owned harness with baselines regenerated from
  measured numbers, comparison against the frozen `aspect_rules_lint`
  v2.8.0 baseline as a report (not a gate)
  ([issue #86](https://github.com/ralvik/rules_dx/issues/86)).
- **Dependency checks**: per-language lockfile-consistency and
  declared-dependency usage fixtures (contract accepted, fixtures open)
  ([issue #22](https://github.com/ralvik/rules_dx/issues/22)).
- **Audit/update live execution**: auditor wiring, advisory acquisition,
  SARIF/SPDX mapping, resolver backends, per-set reporting
  ([issue #18](https://github.com/ralvik/rules_dx/issues/18),
  [issue #19](https://github.com/ralvik/rules_dx/issues/19)).
- **Docs pipeline**: per-language adapter runs, link/reference proofs,
  renderer/site artifacts, guide-step verification
  ([issue #10](https://github.com/ralvik/rules_dx/issues/10)).
- **Environment/codegen**: projection queries, replacement contracts,
  boundary evidence ([issue #9](https://github.com/ralvik/rules_dx/issues/9)).

## Status

`Delivered` means implemented and verified on the Linux x86_64 seed host
only. `Open` means tracked in the linked issue with no implementation
claimed here.

| Language | Corpus dogfood | Layer-2 matrix | Generation | Examples | E2E | Perf | Depcheck | Audit/update | Docs | Env/codegen |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rust | Delivered | Delivered | Delivered | Delivered (`adopt-rust`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Python | Delivered | Delivered | Delivered | Delivered (`adopt-python`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| JavaScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| TypeScript | Delivered | Delivered | Delivered | Delivered (`adopt-js-ts`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Go | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-go`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Java | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-java`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Kotlin | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-kotlin`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Scala | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-scala`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| C# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-csharp`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| F# | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-fsharp`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| C++ | Delivered (code ownership) | Open (adapter-less) | Delivered | Delivered (`adopt-cpp`) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |
| Vue/Svelte/Astro/MDX | Delivered (code ownership) | Open (regions, #8) | Delivered | Open (composition, #8) | Delivered (contract) | Tracked (#86) | Open (#22) | Planning only (#18/#19) | Open (#10) | Open (#9) |

Framework composition evidence (exact parser/compiler, provider,
generated-region, dependency, test, environment/IDE, quality-region
mappings) stays open under
[issue #8](https://github.com/ralvik/rules_dx/issues/8). Language
provider/import/lock/tool-graph proofs stay open under
[issue #7](https://github.com/ralvik/rules_dx/issues/7). Quality
family taxonomy stays open under
[issue #6](https://github.com/ralvik/rules_dx/issues/6).

## Battery

The full close-out battery ([issue #54](https://github.com/ralvik/rules_dx/issues/54))
runs on a clean tree after the staged issues land:

```sh
bazel build //...
bazel test //...
bazel run //cli/cli:dx -- generate --check //...
```

plus the corpus and code ownership audits, the per-harness audits in
`dogfood-freshness`, and the explicit E2E suite. Remaining reds each name
their owning issue above; docs match as-built behavior.
