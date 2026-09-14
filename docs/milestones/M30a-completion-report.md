# M30a Completion Report: Adoption Docs Release-Blocking (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
there is no qualified IR schema, extractor adapter, site-build rule, or `dx
docs` command yet, so no working documentation-delivery support is
established; schema numbers, per-language input pins and adapter mappings,
overload schemes, link/reference completeness, renderer behavior, and
rule/CLI wiring remain unqualified under O54 (`docs/documentation/`,
`docs/cli/commands/docs.md`). What lands here is the pure planning layer
the docs contracts authorize before qualification: IR version/identity, the
extraction-validation gate, check-vs-build modes, the drift upgrade gate,
the guides/examples corpus shape, the site-build action graph, and the `dx
docs` invocation mapping, all in the zero-dep `dx_docs` Rust crate
unit-tested without extractors, toolchains, a Bazel server, or any renderer.

This report closes the planning phase only. Exact `.proto` field/enum
numbers and reserved ranges, per-language adapter mappings and pins,
per-language overload normalization, link/reference completeness proofs,
renderer internals, guide-step text and CI wiring, rule labels,
check/serve combination semantics, site YAML/rules, and CLI registration
are explicitly deferred as qualification-gated follow-ups handed to M28
(see Open Items). The deferral is evidence-backed: the owning contracts
carry a do-not-implement gate until those qualifications land, and no CLI
surface is registered until its behavior lands.

## WP1: Guides And Examples Corpus (planning)

`dx_docs` owns the three frozen release-blocking guide identities before
any prose or CI wiring lands: `quickstart`, `tutorial`, `migration`
(verbatim, no extra guide, no implicit default). Freshness is
CI-executed-or-stale: a guide is fresh only when every step ran in CI and
the `examples/` corpus run stayed green; no step may go unexecuted.
Worked examples live under `examples/` and back guide steps without
remapping onto source or output trees. Exact step text and CI wiring stay
O54-gated (slice 2).

## WP5: Unified Documentation Pipeline And `dx docs` (planning)

IR versioning plans compatibility before any schema freeze:
`doc_ir_version` with nonzero major required; same-major pairs are
compatible in either minor direction (additive-only, extensions preserved
verbatim); major skew requires the recorded migration, never silent
acceptance. Symbol IDs are stable (`language:package:qualified_name`,
workspace-relative paths only) with the overload suffix as trim/empty-drop
normalization; per-language type normalization freezes under O54.
Extraction validation shares one gate for check and build (schema decode,
inventory completeness, ID stability, reference resolution); any failure
fails the action with no partial shard. Check validates without rendering;
build validates then renders; neither compares committed IR and a cache
miss never fails check. Serve previews last build outputs locally with no
own cache and is not a build action. Builds write Bazel outputs only, never
sources; IR shards are never committed. Drift upgrades ship only when the
contract suite, golden fixtures, and determinism evidence are green plus
explicit review; upstream changes never reach users outside a release
(slice 1).

The site-build action graph plans extract (one per language/package unit to
one cached IR shard) to aggregate (shards plus prose plus theme/config with
shared validation to render inputs) to render (pinned mdBook to the static
site tree), with Bazel incrementality as the only rebuild mechanism. Check
selects extract plus aggregate; build adds render; both reject the same
invalid IR and references. Actions declare every input with no network;
outputs are deterministic by construction (sorted order,
workspace-relative paths, no timestamps, no absolute paths, no host
environment, locale-independent, UTF-8). Byte equality is required only
for the same pinned producer and inputs; cross-version compares decoded
semantics. Cache misses re-execute, never fail freshness. Scope stays lazy
with no language enable lists; bare scope selects the repository
(slice 3).

`dx docs` invocation reuses the shared workflow label/pattern/path
resolution with no docs-specific scope syntax. Check and build share one
extraction/aggregation graph, never separate checkers; O54 must prove the
pre-render checks complete. `--port` without `--serve` is rejected.
Failures name the affected (language, package) unit, plus the pinned input
whose schema changed on drift. Check/serve combination semantics stay
O54-gated (slice 4).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (715 targets).
- `bazel test //...`: 180/180 pass, including `//dx/docs:dx_docs_test`
  (29 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate`: no diffs; `bazel run //dx:generate_check`:
  clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_docs` is a zero-dep pure-planning library fully covered by its
co-located unit tests. No schema-number, adapter-execution,
renderer-execution, rule-label, CLI-registration, remote, non-Linux, or
external-consumer evidence; none claimed.

## Changed Components

- `dx/docs/` (crate `dx_docs`): `src/lib.rs` (all four planning slices
  plus 29 unit tests); `BUILD.bazel`, `Cargo.toml` (slice 1).
- Zero-dep by design: no `MODULE.bazel` manifest changes; no `dx/cli`
  registration (behavior has not landed); no site rules or CLI surface.
- This report.

## Open Items

- Qualification gate (O54): freeze IR `.proto` numbers/reserved ranges and
  compatibility fixtures; per-language input pins and adapter-to-input
  reconciliation (thirteen-adapter claim); per-language overload schemes;
  pre-render link/reference completeness; renderer/site layout/URL/search
  mappings; exact `dx docs` invocation/protocol mappings and check/serve
  combos; guide-step text and CI-executed verification wiring; cache and
  determinism execution evidence; nightly rustdoc exception evidence.
- O53/O62 untouched by this report.
- Milestone exclusions respected: no release qualification (M28), no
  publication (M29), no post-release adoption scope (M30b). M28 handoff:
  qualified mappings plus the pure gates above as fixtures for
  clean-external-consumer runs.
