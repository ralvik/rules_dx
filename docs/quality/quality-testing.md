# Quality Workflow Testing

This document defines the required test evidence for quality workflows: what
behavior must be proven and how to run it. Open work is tracked in GitHub
issues. Run suites with `bazel test //...` and bare `dx test`.

## Result Protocol

The normative message and compatibility semantics are defined in
[Quality Result Protocol](quality-result-protocol.md).

Unified-result tests verify that every applicable lint/typecheck/format
target/capability pipeline and audit target/tool action contributes exactly
one result message, target pipelines stay independently cacheable, and no
invocation-wide aggregate result action is created.

Protocol tests verify binary Protobuf round trips, malformed and truncated
input rejection, unknown-field compatibility, required semantic validation,
and bounded per-action fixture sizes. Location fixtures cover UTF-8,
zero-width and end-of-file ranges, malformed boundaries, and per-adapter
coordinate conversion. Path fixtures accept normalized workspace-relative
paths and reject absolute paths, backslashes, empty paths, and escapes.

Generated sources are read-only typecheck context, never direct quality
subjects. Digest fixtures require exact BLAKE3 digests and reject stale
inputs before mutation. Runner fixtures prove fixed-copy and native-edit
tools produce the same normalized edit contract with deterministic ordering.

## Diagnostics And Evaluation

Diagnostic fixtures prove severity thresholds, exit-code interpretation
through parsed diagnostics, and that unexpected exits, signals, launch
failures, malformed output, and protocol failures fail at every threshold.
Default mode fails on warnings and errors and passes on informational
diagnostics.

Fixability is strictly binary: `true` only with an exact same-file candidate
proven to resolve the finding. Rule-level hints, pathless claims, missing
edits, and cross-file associations are unfixable. JSON fixtures retain initial
and terminal findings with explicit snapshot identity; SARIF fixtures prove
check mode describes initial bytes while mutating mode uses terminal findings
for applied files. Evaluator tests prove one consumer per result, no aggregate
action, and that a threshold-only change invalidates evaluators but not
analyzers.

## Initial Adapter Evidence

The [adapter qualification contract](tool-integrations.md#initial-adapter-qualification)
defines per-adapter proof. As-built rules:

- Buildifier: exit zero must not hide incomplete analysis; unreadable inputs
  are omitted with a distinct exit, and lint-fix/format coupling is proven.
- Taplo: fail closed on incomplete parsing and undeclared network inputs;
  ambiguous output fails at every threshold.
- Vale: applicable Vale without a declared config fails with guidance; no
  generated policy, hidden style, or silent omission.
- Repo-owned Markdown checker is check-only with a fixed exit matrix and
  workspace-keyed finding paths; remote URLs are recorded, never fetched.

## Configured Policy

The normative ownership and binding behavior is defined in
[Native Configuration](native-configuration.md).

Default-policy tests assert the exact curated adapter and formatter set for
every policy family and semantic file class against the
[curated baseline](../tools/tool-baseline.md#curated-differences). Override
tests cover additions, removals, generated local bindings, formatter-set
replacement, unknown tools, and edit conflicts. Actions for configured tools
remain absent until applicable targets are analyzed.

Selection tests cover stable tool identifiers, deterministic ordered lists,
and class-aware applicability. They reject unknown or duplicate identifiers,
executable or acquisition overrides, and consumer-constructed adapters.
Omission uses curated defaults; explicit disablement creates no actions,
environment entries, or fetches. Opt-in tools (flake8, pylint, ESLint) stay
unselected unless explicitly chosen.

Native-config tests prove pinned upstream defaults need no repository file
where config-free operation is permitted, while applicable Vale requires
declared config. Explicit config labels are complete action inputs; changing
one invalidates every and only consuming action. Undeclared includes, missing
labels, action-time discovery, home-directory configuration, and conventional
filenames absent from the generated graph cannot affect execution.

Gazelle tests prove config bindings merge only `rules_dx`-owned entries,
package boundaries and visibility follow standard Gazelle behavior, generated
config targets are lightweight and visible to descendant packages, and
`aspect_hints` carry direct `<tool>_config` labels with no aggregator.
Filename recognition accepts only exact documented names. Golden tests prove
no `rules_dx` behavioral preset is merged and config-free adapters match
upstream behavior apart from documented hermetic transport controls.

Environment-parity tests prove every selected adapter exports the exact
executable, runtime, and package closure used by its Bazel actions through
`.dx/bin`; disabled tools are absent. Raw-tool tests prove `.dx/bin`
invocations pass argv, working directory, config discovery, streams, signals,
and exit status through unchanged.

Release tests diff curated policy manifests against the default lifecycle
policy: minor-release lint/audit additions need compatibility qualification,
release notes, and a tested prior-set override; default removals and
formatter-set changes need a major release.

## Cache Correctness

For each check kind, capture action graphs and execution records before and
after controlled changes. Changing one direct source, shared config, tool
version/platform, class membership, or stage subset invalidates exactly the
affected target/capability actions; unrelated actions keep identical keys.
The apply step itself never changes Bazel action keys.

Verification uses `aquery` plus execution logs or a controlled remote cache.
A warm local no-op alone is not a cache test. Delivered evidence is local
execution-log hit/miss; remote-cache proof stays unverified per
[Testing Strategy](../testing/strategy-details.md#remote-tests).

## Determinism

Determinism here is local per-cell determinism only, with no cross-cell union
and no remote claim.

Proven by reordering equivalent declarations, running from different checkout
paths, randomizing query order, multi-owner edits, neutral locale/timezone/
home/PATH, randomized report ordering, reordered tool-selection lists, and
multi-tool permutation checks. Identical inputs produce identical stages,
convergence state, diagnostics, and original-to-final replacements. Oscillation
is reported as oscillation, never false stability.

## Apply Safety

Proven by digest validation before writing, per-stage byte-range validation
against the current virtual snapshot, a complete no-change round with a fixed
ten-round limit and one original-to-stable candidate, identical final bytes
across owner/configuration pipelines, rejection of invalid UTF-8, overlapping
or no-op edits, atomic per-file commits with deterministic path order, no
post-apply Bazel rerun, and complete deterministic diff rendering from the
same validated edit set. Check mode fails on any proposed change and never
writes. One rejected path never blocks valid unrelated paths.

## Tool Parity

Each supported language needs separate consumer fixtures for lockfile
consistency and declared-dependency usage. A stale lockfile fails consistency
even when every declaration is used; a consistent lockfile with an unused
declaration passes consistency but fails usage. Tests run offline after
declared inputs are provisioned, without querying live registries, and neither
test mutates manifests or locks. Exceptions need explanatory reasons; obsolete
exceptions fail validation.

Every required entry in [First-Release Tool Baseline](../tools/tool-baseline.md),
including mandatory curated expansion, requires a fixture with native
configuration, passing and failing diagnostics, supported platform coverage,
and applicable fix/format output. A generated parity manifest fails CI when a
required entry lacks its tests. Tool-update tests regenerate versions and
checksums, reject missing platform artifacts, and run the affected adapter
suite.

Cross-cutting fixture, remote, and acceptance-evidence requirements remain in
[Testing Strategy](../testing/README.md). Acquisition, laziness, and platform
requirements are in [Tool And Platform Testing](../testing/tools.md).
