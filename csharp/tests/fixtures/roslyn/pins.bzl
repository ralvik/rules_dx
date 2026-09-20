"""Roslyn per-TFM-RID SARIF aggregation decision (issue #492).

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`,
`docs/product/support-matrix.md#provisional-adapter-input-notes`.

Decides the open Roslyn adapter-input risk with fixture evidence, recorded
explicitly here and in the owning docs, never silently dropped. The
single-SARIF assumption is rejected per the issue alternatives.

Upstream facts (observations, not pins; recheck the qualified .NET SDK at
implementation):
- Roslyn `/errorlog:<file>,sarif` emits SARIF 2.1 per `csc` compiler
  invocation. Each configuration/TFM/RID pivot is a separate invocation
  with its own SARIF file carrying its own single `runs[0]` entry
  (`tool.driver.name == "csc"`; see `net8.sarif` plus `net10.sarif`).
- Different pivots carry different result sets: TFM-gated code (`#if
  NET10_0` in `Sample.cs`) means the net10.0 pivot reports CA1303 while
  the net8.0 pivot does not; both pivots report the shared CA1822. A
  reader that keeps one pivot SARIF silently drops the TFM-specific
  finding, so aggregation must be a union, never a pick-one.
- Roslyn is SDK-coupled with no separate artifact: the compiler plus its
  CA analyzers follow the qualified .NET SDK (pinned coupling in
  `scala/tests/fixtures/scala_dotnet_quality/pins.bzl` under issue #486;
  digests stay owned under issue #417).
- `artifactLocation.uri` values must be re-rooted to workspace-relative
  slash-separated paths (`csharp/tests/fixtures/roslyn/Sample.cs`); absolute
  paths, `file://` URIs, and Bazel execution paths are never source
  identities per the result protocol.

Decision (adapter-only, no `csharp` claim yet; cohort stays owned by #417):
- Single-SARIF assumption REJECTED. The adapter must never read one pivot
  SARIF and claim full-target coverage; a missing declared pivot SARIF
  fails the action closed, never becomes an empty successful result.
- Collection REQUIRED: every declared per-pivot `/errorlog` SARIF is a
  declared action input resolved from the authoritative `csharp_*` target
  context (never an ambient filesystem scan, never inferred). Each file
  must parse as SARIF 2.1 with the expected single-run shape; malformed,
  truncated, version-mismatched, or driver-mismatched output fails the
  action, never becomes findings or an empty result.
- Aggregation REQUIRED: concatenate the per-pivot `runs` arrays into one
  SARIF log with a single `$schema` plus `version` (`2.1.0`), runs in
  deterministic pivot order (ascending TFM version, then RID, then
  configuration; see `aggregated.sarif` with
  `csharp-roslyn/net8.0-linux-x64/Debug` before
  `csharp-roslyn/net10.0-win-x64/Debug`, since plain string sort would
  order `net10.0` before `net8.0`). Each run is preserved verbatim
  except `artifactLocation.uri` re-rooting to workspace-relative form.
  The union keeps shared findings once per producing pivot (CA1822 twice)
  plus TFM-specific findings (CA1303 once); nothing is dropped.
- Merged-single-run REJECTED: collapsing all results into one run loses
  per-pivot provenance (which configuration/TFM/RID produced which
  finding) and conflates TFM-gated diagnostics. Runs concatenation keeps
  `automationDetails.id` plus `properties.{configuration,tfm,rid}`
  provenance per run. Deduplication happens only when normalizing to the
  result contract (identical exposed fields dedup per
  `docs/quality/quality-result-protocol.md`; a deduplicated finding is
  fixable only if every contributing pivot context guarantees it).
- Check-only with the provisional sandbox-apply-and-diff fix flow (Roslyn
  produces diagnostics; fixes travel as unified patches, never `IN_PLACE`
  mutation of immutable inputs).
- Native config is sole policy (unchanged from issue #486): SDK default
  analysis mode is the upstream built-in default; StyleCop stays opt-in.
"""

# SDK coupling (qualified seed-only under #486; digests stay owned under
# issue #417; living at head rejected).
ROSLYN_COUPLING = "SDK-built-in CA analyzers following the qualified .NET SDK, no separate acquisition"

# Wire shape (upstream `/errorlog` contract, not a rules_dx invention).
ROSLYN_SARIF_VERSION = "2.1.0"
ROSLYN_ERRORLOG = "/errorlog SARIF 2.1 per csc invocation (one run per file)"
ROSLYN_DRIVER = "csc"

# Explicit aggregation record (single-SARIF assumption rejected).
ROSLYN_AGGREGATION = "concatenate per-pivot runs into one SARIF log with single $schema plus version; union of results with per-pivot provenance; single-SARIF assumption rejected"
ROSLYN_RUN_ORDER = "deterministic pivot order: ascending TFM version then RID then configuration (net8.0-linux-x64 before net10.0-win-x64; plain string sort would invert net10/net8)"
ROSLYN_PROVENANCE = "per-run automationDetails.id plus properties.{configuration,tfm,rid}; artifactLocation.uri re-rooted to workspace-relative"
ROSLYN_COLLECTION = "every declared per-pivot /errorlog SARIF is a declared action input from the authoritative csharp_* target context; missing or malformed fails closed"
ROSLYN_MERGED_RUN_REJECTED = "merged-single-run rejected: loses per-pivot provenance and conflates TFM-gated diagnostics"
ROSLYN_SINGLE_SARIF_REJECTED = "single-SARIF assumption rejected: one pivot SARIF never claims full-target coverage"
ROSLYN_FIX_FLOW = "check-only with sandbox-apply-and-diff unified patches; IN_PLACE mutation rejected under sandboxing"

# Native-config sole policy (no hidden preset; adapters transport-only).
ROSLYN_CONFIG_POLICY = "SDK default analysis mode is the upstream built-in default, StyleCop stays opt-in"

# Fixture pivots (evidence for the union proof: shared CA1822 in both
# pivots, TFM-specific CA1303 only in the net10.0 pivot).
ROSLYN_FIXTURE_PIVOT_NET8 = "csharp-roslyn/net8.0-linux-x64/Debug"
ROSLYN_FIXTURE_PIVOT_NET10 = "csharp-roslyn/net10.0-win-x64/Debug"
ROSLYN_FIXTURE_SHARED_RULE = "CA1822"
ROSLYN_FIXTURE_TFM_SPECIFIC_RULE = "CA1303"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet under issue #417, decision recorded under issue #492).
CSHARP_FIXTURE_HELLO = "//csharp/tests/fixtures/hello:hello_test"

# Rejected: single-SARIF coverage plus merged runs plus ambient collection.
ROSLYN_REJECTED = "single-SARIF assumption rejected: must record; merged-single-run rejected; ambient SARIF discovery rejected; IN_PLACE patching rejected"
