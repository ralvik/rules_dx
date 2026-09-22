# Quality Action Model

## Working Model

Aspects attach capability pipeline actions to existing source-owning targets. Each applicable
target gets one lint pipeline and one format pipeline containing its complete selected tool
set of nonempty stages; compatible Python targets get one typecheck pipeline containing Ty and any other selected
type checkers. Audit tools may remain independent target/tool actions because they do not
mutate. There is no repository-wide action by default and no action per individual file by
default.

Workspace ecosystem tool-ID selection, expanded through ruleset-owned adapter manifests into
derived class-aware policy, supplies the tool set rather than the action boundary.
`python_library`, `python_binary`, and `python_test` targets each receive separate
applicable pipelines. Custom rules expose any number of direct-source classes through
`QualitySourcesInfo`; those classes determine each adapter's source subset without creating
repository-, language-, or class-wide actions.
The aggregate workspace provider expands ecosystem tool selections through ruleset-owned
adapter manifests into class-aware policy; users do not maintain extension or class lists.

This model is exercised through the repository corpus (`real_source_target(name = "corpus_*")`
per content type per package, issue #15, e.g. `dx/BUILD.bazel`) via the corpus dogfood in
[local workflows](../contributing/local-workflows.md#corpus-dogfood). Remote execution
remains unverified. [ADR 0003](../decisions/0003-action-granularity.md)
is accepted (issue #756 per [ADR 0022](../decisions/0022-no-benchmarking.md));
cross-target batching stays rejected.

## Why Target-Level Actions

- A source target expresses ownership and semantic direct-source classes through an authoritative
  supported provider contract.
- A source change invalidates only checks whose declared inputs include it.
- Package-sized actions amortize process startup better than per-file actions.
- Target/capability pipelines remain independently schedulable and cacheable.
- Reports and replacements map back to familiar Bazel labels.

The cost is each nonempty selected stage startup, and potentially multiple convergence rounds, per
applicable target on a cold execution. A large
graph with many tiny targets pays per-target startup; cross-target batching is rejected
by fiat per [ADR 0022](../decisions/0022-no-benchmarking.md) (wont-fix, issue #756).

## Ty Boundary

The initial Ty model mirrors the v2.8.0 reference shape: visit wrapped
`python_binary`, `python_library`, and `python_test` targets, propagate over `deps`,
check each target's direct sources, and include transitive sources, stubs, and
import roots exposed by verified `aspect_rules_py` providers. This is graph-native
and incremental, but can duplicate analysis across small targets. The boundary
is decided by reasoning per [ADR 0022](../decisions/0022-no-benchmarking.md):
action count, repeated inputs, cache reuse, and diagnostic quality are compared
by fixture shape, not wall time.

## Action Inputs

A non-mutating check declares all inputs that can affect its result:

- Direct source files and any provider-derived dependency closure required by the
  adapter. Writable direct subjects are limited to that adapter's effective semantic-class
  subset; dependency and generated context remains read-only.
- The pinned checker executable and runtime files.
- Effective native configuration files and their complete declared dependency closures, as defined
  by [Native Configuration](native-configuration.md).
- Generated sources and import metadata where applicable.
- Deterministically ordered rule options.
- Platform and toolchain selection encoded by Bazel.

Ambient home-directory configuration, current working directory discovery, PATH
tools, network access, and undeclared repository files are prohibited for
hermetic checks.

## Native Configuration Boundary

Actions consume only explicitly bound native configuration and its complete declared closure.
Ownership, Gazelle discovery and binding, per-file resolution, exclusions, invalidation, and stale
binding behavior are defined in [Native Configuration](native-configuration.md). Changing one target
source invalidates every applicable owner action that declares it and each Ty action whose declared
source or dependency closure includes it.

## Deterministic Arguments

Rules sort labels and paths using a documented stable ordering before constructing
action arguments. Workspace-relative keys (`short_path`) are the stable cache identity;
exec-root values (`File.path`) locate bytes at execution time. Stage order follows the
curated adapter-registry definition order, independent of user list order. Config closures
are sorted by `short_path` before emitting `--tool-file` entries; sources, siblings, and
resolve entries are sorted likewise. User-authored ordered option lists retain semantic
order. Sets or depsets are converted with an explicitly selected order. Tests compare action
keys or action descriptions across declaration-order permutations.

## Tool Pinning

Tool versions and artifact integrity are part of Bzlmod/toolchain configuration,
not CLI state. Integrations consume `rules_dx`-supplied ready-to-run tools under
[Tool Acquisition](../tools/tool-acquisition.md). Each adapter exposes the resolved upstream,
runtime where applicable, and delivery identity for diagnostics.

## Outputs, Remote Cache, and Execution

Each pipeline or audit action produces a versioned unified result message in the `dx_results` output
group. It contains normalized diagnostics and execution status and may contain
proposed inline byte-range replacements. Result messages are ordinary Bazel
artifacts and never write the workspace during action execution.

The exact message and apply semantics are defined in
[Quality Result Protocol](quality-result-protocol.md).

One target/capability pipeline produces one result message; audit may produce target/tool
messages. No invocation-wide aggregation action is added; `dx` performs deterministic result
consensus and merging after
discovering artifacts through BEP.

Each result is a single binary Protobuf message. Incremental processing occurs
across independently completed action artifacts, not within one message.

Hermetic actions are cacheable and local-only until remote is qualified:
every Dx pipeline plus evaluator action carries `no-remote-exec`, so Bazel
never schedules them on a remote executor (wrong-platform remote exec is
rejected by construction) while local disk-cache reuse still applies.
Determinism claimed here is local per-cell determinism only, with no cross-cell
union and no remote claim: per-host pinned binaries with no qualified remote
platform; paid remote services stay unapproved per the testing README
infrastructure budget and Apple/MS cache rights stay license-bounded.
Platform-specific binaries produce platform-specific action keys. Rules must
not drop the marker merely for convenience. If Ty lacks a supported hermetic
route on a required platform, the first-release parity gate remains blocked
unless the baseline is explicitly changed.

Remote boundary: pipeline plus evaluator actions are safe to cache locally
(deterministic inputs plus versioned result messages) but not safe to
execute remotely today — tool artifacts are per-host pinned binaries with no
matching remote execution platform qualified, and no remote cache or executor
is wired. Qualification would require pinning toolchains/hubs as toolchain inputs with
matching remote platforms, not loose file inputs; until then `no-remote-exec` stays
fail-closed (the hermetic scratch in `quality/runner/src/real.rs` alone does not qualify
remote). Local execution-log hit/miss is the delivered cache evidence; a warm
local no-op alone is not a cache test, and controlled remote-cache proof stays
unverified. Lightweight proxy budgets (action count plus cache-hit, no wall-time benchmarks
per [ADR 0022](../decisions/0022-no-benchmarking.md)) bound scale: clean/warm/one-edit/shared-config
counts plus per-fixture exact action counts in `quality_cache_aquery` prove fan-out without
reviving full benchmarks. Only the final Rust apply step updates the local
working tree. It validates complete result envelopes, groups edits by path, and atomically
applies each valid file independently. Its design is covered by
[ADR 0005](../decisions/0005-mutating-operations.md).

Evidence: `bazel run //tools/ci:action_execution_cache_qualification`
(aquery `ExecutionInfo` plus `ActionKey` shape plus local execution-log
hit/miss; remote stays unverified).

## Aggregation

A Bazel invocation applies configured aspects to selected source targets and
requests `dx_results`. This collects independent per-target/capability actions
without combining their inputs or changing their action keys. `dx` reads artifact
locations from Bazel's Build Event Protocol rather than scanning output trees.

## Capability Semantics

Many linters may apply to the same target and source file. They run as ordered stages inside one
target lint pipeline so later stages can consume earlier virtual fixes on overlapping effective
subsets and complete rounds can converge without writing the workspace. Disjoint stage sources
remain in the same target/capability action but are never passed to an inapplicable adapter.

Lint pipelines produce normalized initial and terminal quality results plus original-to-stable
byte-range edits. `dx lint` collects all selected outputs and applies each valid file atomically
once. Oscillation, iteration limit, owner-context disagreement, stale bytes, or path-local
validation failure leave that file unchanged while valid unrelated files apply. Any rejected file
fails the command. Result collection or envelope failure before the candidate path set is known
writes nothing. The CLI does not rerun affected checks after apply; iteration occurs only inside the
Bazel action's virtual source state. `dx lint --check` performs the same edit normalization and
validation but emits exact proposed changes instead of applying them.

Type checkers, including Ty, belong to `dx typecheck`; adapters that cannot produce fixes contribute
diagnostics only. Typecheck fixes use the same replacement and per-file atomic apply protocol as lint
fixes. Fix-capable pipelines retain initial and terminal diagnostics and set binary fixability only
for initial findings proven absent at convergence because of their exact same-file candidate. The CLI
applies valid replacements once and does not automatically rerun any selected type checker. Findings
not resolved by an applied guaranteed fix at or above `--fail-on`, or apply failures, determine a
nonzero result. Suppression insertion is excluded from normal fixing. Security analyzers belong
to `dx audit` and are not selected by lint or typecheck; Python source-audit selection is
qualified seed-only under closed #801 (`python/tests/fixtures/python_audit/pins.bzl` with
`python_audit.expected` via `bazel run //tools/ci:python_audit_qualification` with Ruff S
selected via the pinned Ruff standalone artifact plus curated audit empty with explicit
disablement; Bandit excluded from v1 by
[ADR 0019](../decisions/0019-first-release-additional-foundations.md) and re-selected by
[ADR 0032](../decisions/0032-ruby-powershell-bandit-swift.md) with wiring pending under #801; family taxonomy execution
qualified seed-only under closed #512 via `bazel run //tools/ci:quality_taxonomy_qualification`,
closed #512 stays taxonomy-only).

Several active formatter implementations may claim the same file. They run as stable ordered stages
in one target format pipeline, each over its fixed effective subset. A complete round succeeds only
when every nonempty formatter stage leaves its subset unchanged; the final stage is not an
unconditional winner. A repeated changed state or continuing edits at the round limit fail
convergence. Other valid files may still apply through agreeing pipeline results; any rejection fails
the command. Changing the formatter set, order policy, or one formatter or config changes affected
action keys.

Formatting applies only to files owned by Bazel targets. A format aspect creates cacheable target
pipeline actions that emit validated original-to-stable byte-range edits without writing the
workspace. `dx format` atomically applies each file after checking its recorded input digest.
`dx format --check` emits those same normalized replacements as exact proposed changes and never
writes files. This intentionally differs from `aspect_rules_lint`, whose mutating formatter can walk
the working tree and format files unknown to Bazel. Requiring ownership keeps Bazel authoritative and
preserves target-scoped caching and local execution (remote remains unqualified).

## Required Measurements

The granularity rule is accepted by reasoning and fiat per
[ADR 0022](../decisions/0022-no-benchmarking.md) and
[ADR 0003](../decisions/0003-action-granularity.md); fixtures below prove
correctness, convergence, and cache shape, not wall-time benchmarks:

- Clean local execution action count and checker startup share.
- Warm local no-change execution and action count.
- One-source edit: executed versus cache-hit actions.
- Shared-config edit: expected fan-out.
- Cross-tool convergence passes, rounds and process starts, and pipeline-key invalidation when one
  selected tool or config changes.
- Mixed-class targets with disjoint and overlapping stage subsets, including omission of empty
  stages and invalidation when class membership changes.
- Permutation comparisons for each supported multi-tool set: convergence outcome, terminal
  bytes/findings, rounds, and process starts; wall time is not measured per ADR 0022.
- Remote cache upload/download behavior across clean output bases.
- Remote execution scheduling shape where infrastructure exists; wall time is not measured.
- Diagnostics and failure isolation with multiple failing targets.

Tests must inspect `aquery` action inputs and use execution logs or a controlled
remote cache. A successful second command alone is not evidence of correct cache
invalidation.
