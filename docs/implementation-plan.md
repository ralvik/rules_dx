# Implementation Plan

## Delivery Flow

Milestones are planned in dependency order. Dependency readiness is not authorization to begin
implementation; entry gates and completion requirements live in the [milestone index](milestones/README.md).
Agent execution procedure lives in the [repository instructions](../AGENTS.md) and entry gates
in the [common entry definition of done](milestones/README.md#common-entry-definition-of-done).
A failed feasibility gate blocks affected first-release scope until a scope or design decision resolves it. Decisions
are recorded in their owning ADR or domain contract; milestone completion is recorded in that
milestone's completion report.

The accepted [consumer GitHub CI contract](github-ci.md) has delivery work packages assigned to
[M27](milestones/M27-consumer-ci.md), with release requalification in M28 and public-delivery
verification in M29. This planning assignment changes no milestone statuses or dependencies.
Exact workflow APIs/pins, runner/platform mappings, event/ref bindings, security/permission
boundaries, and review-thread mechanics/limit mappings must be qualified and frozen before
affected M27 implementation; M28 is not a deferral of that gate.

## Rationale

The Moderate DAG delivers a narrow Bazel/Rust quality path against this repository before expanding
the CLI, generators, language foundations, frameworks, parity tools, and repository workflows. This
dogfood-first order tests shared contracts with real repository use while keeping each completion
report reviewable.

## Phase Overview

- M00-M07 establish bootstrap infrastructure, shared quality contracts, direct-Bazel dogfood, and the
  initial quality CLI.
- M08-M13 add target resolution, Rust-first generation, public generation, environments, the complete
  Rust foundation, and repository-corpus closure.
- M14-M21 add Python, JavaScript/TypeScript, Vue, Svelte, Astro, MDX, mixed-framework behavior, and
  provider-derived language-target environment plans/projections; M25 owns public env orchestration.
- M22/M23 own admitted additional-foundation cohorts and unchanged tool parity;
  M24 closes the admitted inventory under [first-release admission](product/scope.md#first-release-admission).
  O46 must first freeze inventory, mappings, effort evidence, and reviewable work packages; dispositions
  live in [ADR 0019](decisions/0019-first-release-additional-foundations.md). This
  plan by itself selects or defers no additional language and starts no milestone.
- M25-M29 complete repository workflows and reusable consumer GitHub CI, qualify release policy,
  CI revisions, and exact bytes, then execute recorded publication and public-consumer verification.
  The release-blocking documentation subset (M30a) is consumed by M28; remaining adoption scope
  stays post-release (M30b) under O54.
- M30 turns publication into adoption: onboarding, scaffolding, hermetic hooks, devcontainer,
  diagnostics, `dx` versioning, and unified documentation delivery. O49/O50/O51/O54 freeze hook,
  diagnostics, version, and documentation mechanics before affected M30 implementation.

## Milestone Index

The sole source for milestone order, status, dependencies, shared definitions of done, and completion
report requirements is the [Milestone Index](milestones/README.md).

Known prerequisite and release-sequencing contradictions are recorded as
[cross-document blockers](open-decisions.md#cross-document-blockers). This plan does not resolve them
or authorize affected implementation.
