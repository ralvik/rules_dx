# M20: Astro

## Outcome

Astro components have an adapter-tested application, generation, test/coverage, quality, IDE, and
provider-derived environment-plan path for focused targets.

## Scope

Implement an Astro-specific adapter over its pinned upstream parser/compiler and Bazel integration, including
frontmatter/server script, component/template regions, client scripts, dependencies, generated outputs,
test/coverage ownership, and focused environment projections. Public repository/root/exact-target env
orchestration remains in M25.

## Contract References

- [Generation](../generation/), [quality](../quality/), [environments](../environments/), [tools](../tools/), [support matrix](../product/support-matrix.md), [testing](../testing/), and open decision [O41](../open-decisions.md).

## Deliverables

- Astro wrappers/provider normalization, Gazelle adapter, quality classification, and provider-derived
  environment/IDE plan and focused projection.
- Astro build, test, coverage, run, strict dependency, merge, platform, and laziness fixtures.

## Work Packages

1. Prove the stable upstream Astro parser/compiler, package, provider, and target boundary.
2. Implement physical ownership, region hand-off, generation, execution, test/coverage,
   provider-derived environment plans/projections, and quality integration.
3. Certify client/server semantics, stale cleanup, collisions, unsupported syntax, and laziness.

## Milestone-Specific Evidence

- Frontmatter, template, and client regions retain authoritative semantics without regex extraction or duplicate ownership.
- Astro fixtures prove build, test, coverage ownership/completeness, and focused environment projection
  for every applicable support-matrix cell independently before any mixed-framework claim.

## Out Of Scope

- Vue, Svelte, MDX, generic container APIs, and supported status.

## Completion Report Additions

- Record upstream stack, supported Astro syntax/targets, provider mappings, regions, platforms, and limitations.
