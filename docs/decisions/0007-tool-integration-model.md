# ADR 0007: Tool Integration Model

## Status

Accepted.

## Context

The platform needs broad linter, formatter, type-checker, and audit coverage
without creating a separate public architecture for every tool.
`aspect_rules_lint` v2.8.0 demonstrates a useful tool matrix and aspect-based
lint model, but depending on it would couple `rules_dx` to another public API
and implementation lifecycle. Its working-tree formatting model also does not
provide the target ownership and graph-defined execution required here.

Tools have materially different distribution shapes. Some belong to a selected
language toolchain, some ship complete native artifacts, some require a locked
language package graph, and others require a release-assembled runtime closure.
Forcing those tools through one acquisition mechanism would either leak setup to
consumers or weaken hermeticity and platform support.

Quality commands also need one consistent execution and result model. Bazel and
the `dx` CLI must evaluate the same analyzer output, while fix-capable tools must
compose without writing intermediate states to the workspace.

## Decision

### Ownership And Scope

`rules_dx` implements integrations independently with project-owned Starlark
aspects and project-owned Rust runners. It neither depends on
`aspect_rules_lint` nor promises compatibility with that project's public APIs.

The frozen `aspect_rules_lint` v2.8.0 tool list is a minimum first-release
baseline, not a scope ceiling or an API or behavioral parity promise. Curated
expansion to feasible quality tools and features across languages is mandatory where
upstream implementations or Bazel rules permit hermetic thin integration under
[First-Release Admission](../product/scope.md#first-release-admission).
This does not authorize building replacement stacks.

Swift and SwiftFormat are excluded from v1 by
[ADR 0019](0019-first-release-additional-foundations.md); host
toolchain fallback was never approved. The exclusion is recorded in
[Swift Feasibility](../tools/tool-baseline.md#swift-feasibility); reconsidering Swift after v1
requires a new scope decision.
The integration policy and authoritative baseline are maintained in
[Tool Integrations](../quality/tool-integrations.md) and
[First-Release Tool Baseline](../tools/tool-baseline.md).

Public APIs are capability-oriented rather than tool-oriented. Adding a tool
normally adds an internal adapter and does not add a public rule family, CLI
surface, or per-tool result contract. A new public tool-specific API requires a
separate demonstrated need.

### Acquisition

Every managed tool is supplied as a pinned, hermetic, ready-to-run Bazel
dependency. Its internal delivery class follows its authoritative upstream
shape:

- compiler-coupled tools use the selected language toolchain;
- independent complete tools use checksummed artifacts;
- interpreted tools may use private `rules_dx`-owned package locks through
  established Bazel language rules; and
- exceptional runtime closures are assembled in release CI.

These classes share one adapter boundary and do not change quality-result
semantics. Consumers do not select an acquisition class or install, compile, or
resolve managed quality tools. [Tool Acquisition](../tools/tool-acquisition.md)
owns delivery, pinning, platform, update, and proof behavior.

Retaining these delivery classes does not authorize replacement stacks. Language integration
maintenance follows [Product Scope](../product/scope.md#language-integration-maintenance);
candidate-specific routes and effort still require review under O46 in the
[open-decision register](../open-decisions.md).

### Actions And Results

Capability aspects attach quality work to existing Bazel targets. The actions
are graph-owned and may produce diagnostics and proposed source replacements;
they never mutate the workspace. Exact granularity, inputs, capability
boundaries, and aggregation are owned by the
[Quality Action Model](../quality/action-model.md) and remain provisional where
that document says they are provisional.

All capabilities use one versioned quality-result contract and the single
`dx_results` output group. The `dx` CLI discovers and consumes those results
through Bazel's Build Event Protocol. Direct Bazel use consumes the same results
through Bazel-owned evaluator actions, so analyzers do not encode either CLI or
CI severity policy. The [Quality Result Protocol](../quality/quality-result-protocol.md)
owns the schema, execution classification, evaluation, collection, and mutation
semantics.

### Composition

Fix-capable tools compose through bounded virtual convergence inside Bazel
actions: stages observe prior virtual changes and must reach a shared stable
state before a workspace replacement is eligible. They do not run as sequential
writers against the real workspace, and a final tool is not an unconditional
last-writer winner.

Stage order is deterministic, versioned ruleset policy. It is not user
declaration order, lexical tool-ID order, target configuration, or a public
priority mechanism. The action model and result protocol own the convergence
algorithm, limits, candidate consensus, and apply behavior.

Integration acceptance and support claims require the evidence defined by the
[Testing Strategy](../testing/README.md) and its focused test matrices;
documentation or inclusion in the baseline is not proof of implementation.

## Consequences

- Shared capability contracts replace per-tool public rule families and result
  formats.
- The project owns adapter, Starlark, runner, and compatibility maintenance
  instead of inheriting it from `aspect_rules_lint`.
- Tool-list parity is a substantial implementation, platform, and testing gate,
  while tool versions may advance independently from the v2.8.0 baseline.
- Target attachment keeps source ownership and execution in the Bazel graph;
  unowned working-tree files are outside this model.
- One result contract lets the CLI and direct Bazel evaluation apply consistent
  policy without rerunning analyzers through separate integrations.
- Virtual convergence permits compatible tools to compose and rejects tool sets
  that cannot establish a common stable state.
- Delivery classes preserve a uniform consumer experience without forcing
  unlike upstream tools into one packaging mechanism.
- New integrations usually change internal adapters, acquisition metadata, and
  fixtures rather than public APIs or CLI code.

## Rejected Alternatives

- Depending on, forking, or matching the public APIs of `aspect_rules_lint`.
- Exposing a public rule family, result format, or CLI API for each tool.
- Scanning the working tree or providing an uncached fallback for unowned files.
- Running fix-capable tools as sequential writers against the real workspace.
- Treating the last configured tool as the winner without proving a shared
  stable state.
- Letting users or lexical tool identifiers determine stage precedence.
- Giving analyzer processes separate result paths for CLI and direct Bazel use.
- Requiring consumer-authored package graphs, consumer-side installation or
  compilation, ambient PATH discovery, or action-time downloads.
- Repackaging every managed tool into one uniform archive or executable shape.
