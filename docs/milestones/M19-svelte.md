# M19: Svelte

## Outcome

Svelte components have an adapter-tested application, generation, test/coverage, quality, IDE, and
provider-derived environment-plan path for focused targets.

## Scope

Implement a Svelte-specific adapter over its pinned upstream parser/compiler and Bazel integration, preserving
one physical owner and exact script, template, style, dependency, generated-output, test/coverage,
quality-region, and focused environment projection semantics. Public repository/root/exact-target env
orchestration remains in M25.

## Contract References

- [Generation](../generation/), [quality](../quality/), [environments](../environments/), [tools](../tools/), [support matrix](../product/support-matrix.md), [testing](../testing/), and open decision [O40](../open-decisions.md).

## Deliverables

- Svelte wrappers/provider normalization, Gazelle adapter, quality classification, and provider-derived
  environment/IDE plan and focused projection.
- Svelte build, test, coverage, run, strict dependency, merge, platform, and laziness fixtures.

## Work Packages

1. Prove the stable upstream Svelte parser/compiler, package, provider, and target boundary.
2. Implement physical ownership, virtual-region hand-off, generation, execution, test/coverage,
   provider-derived environment plans/projections, and quality integration.
3. Certify stale cleanup, collisions, unsupported syntax, and unused-adapter laziness.

## Milestone-Specific Evidence

- No generic-container parsing, plain-JS fallback, duplicate region ownership, or generation-time compilation occurs.
- Svelte fixtures prove build, test, coverage ownership/completeness, and focused environment projection
  for every applicable support-matrix cell independently before any mixed-framework claim.

## Out Of Scope

- Vue, Astro, MDX, generic container APIs, and supported status.

## Completion Report Additions

- Record upstream stack, supported Svelte syntax/targets, provider mappings, regions, platforms, and limitations.
