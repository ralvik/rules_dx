# ADR 0019: First-Release Additional-Foundation Dispositions

## Status

Date: 2026-09-08.

Accepted. Records the scope dispositions summarized in the
[support matrix](../product/support-matrix.md#candidate-disposition-status).
It complements [ADR 0016](0016-broad-first-release.md), which sets direction
but approves no milestone by itself.

## Context

[ADR 0016](0016-broad-first-release.md) requires broadly working v1 languages
when existing upstream rules permit low-cost hermetic integration, and defers
each admission to the [first-release admission
policy](../product/scope.md#first-release-admission). The support-matrix
candidate reviews (native review 2026-09-07, language-foundation review
2026-09-08) identified concrete upstream routes for each candidate but, as
reviews, admitted, deferred, or excluded nothing by themselves. M22/M23 work
packages need a durable disposition so qualification has something to implement.

## Decision

The following dispositions are v1 scope decisions under the admission policy:

- Admitted to v1: Java, Kotlin, C#, F#, Go, C/C++, and Scala application
  foundations. Each has an active Bzlmod-published upstream ruleset with a
  concrete dependency-lock and toolchain story, documented in the
  [candidate review](../product/support-matrix.md#additional-language-foundation-candidate-review).
  Provisional upstreams, exact versions, mappings, and adapter work stay under
  O30/O31 qualification; this record freezes only the admit outcome.
- Deferred beyond v1: Ruby and PowerShell application foundations. Ruby's
  fragmented ruleset maintenance ownership and gem/bundler packaging effort,
  and PowerShell's single young execution-only upstream with unproven
  generation, dependency, environment, and IDE stories, exceed the low-cost
  hermetic bar. Their quality-tool cohorts (RuboCop, StandardRB,
  PSScriptAnalyzer) stay v1 scope under O31; a foundation deferral removes no
  baseline tool.
- Excluded from v1: Swift/SwiftFormat and Bandit. These are evidence-backed
  exclusions, not pending assessments. Reconsideration after v1 requires a new
  scope decision.

Delivery cohorts are unchanged:

- Go and C/C++ in M22 under O30.
- Java, Kotlin, C#, and F# in M23 under O31.
- Scala after the M22 native/toolchain-versus-managed route decision (O30),
  then M22 or M23.
- Ruby and PowerShell tool cohorts stay in M23 under O31; their foundations
  are out of v1 scope.
- Unresolved cells block qualification. Moving an admitted foundation out
  later requires a new evidence-backed decision.

## Consequences

- M22/M23 may implement admitted foundations once O46/O30/O31 freeze
  per-language contracts, mappings, effort evidence, and reviewable work
  packages. This record alone starts no milestone.
- The quality-tool baseline is independent of foundation deferral.
- O46 tracks per-candidate mappings, effort, and ownership; O30 owns
  native/toolchain qualification detail; O31 owns managed-runtime detail.
- Newly identified candidates still require an explicit disposition under the
  admission policy; this record admits no unnamed language.

## Rejected Alternatives

- Blanket post-v1 deferral of all additional languages: rejected; it
  contradicts the accepted broad-first-release direction.
- Admitting Ruby/PowerShell foundations despite the cost evidence: rejected;
  deferral with retained tool cohorts preserves the baseline without delaying
  the core.
- Leaving Swift and Bandit as perpetually pending: rejected; the evidence
  supports explicit exclusion with a defined reconsideration path.
