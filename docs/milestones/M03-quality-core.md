# M03: Quality Source, Policy, Result, Runner, And Evaluator Core

## Outcome

One target-scoped quality core can declare sources and policy, execute a tool, emit normalized results,
and enforce them directly in Bazel.

## Scope

Implement `QualitySourcesInfo`, semantic classes, typed workspace policy, capability dispatch,
`dx_results`, the versioned result protocol, Rust runner, and Bazel-owned evaluator without real tool
adapters.

## Contract References

- [Action granularity](../decisions/0003-action-granularity.md), [configuration](../decisions/0011-configuration-composition.md), [quality](../quality/), [architecture](../architecture/), [testing](../testing/), and open decisions [O17, O18, and O19](../open-decisions.md).

## Deliverables

- Experimental capability aspects, source/config providers, generated class registry, runner, and evaluator.
- Protocol compatibility fixtures and deterministic synthetic adapter.

## Work Packages

1. Freeze source, policy, applicability, and result schema boundaries.
2. Register exact-input target/capability actions and normalized result production.
3. Add threshold/check evaluators and synthetic end-to-end fixtures.

## Milestone-Specific Evidence

- Analysis proves exact direct-source subsets, no generic attribute fallback, and no empty actions.
- Runner/evaluator fixtures prove deterministic diagnostics, edits, failures, and threshold-only invalidation.

## Out Of Scope

- Real quality tools, BEP collection, workspace mutation, CLI commands, and native-config generation.

## Completion Report Additions

- Record schema field allocation, public load labels, evaluator semantics, and unresolved granularity measurements.
