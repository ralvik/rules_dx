# M21: MDX And Mixed Framework

## Outcome

MDX is adapter-tested for application, test/coverage, quality, IDE, and provider-derived environment
plans, and all v1 framework adapters interoperate in mixed packages and dependency graphs.

## Scope

Implement MDX through its pinned upstream compiler/rules integration, then certify Vue, Svelte, Astro, MDX,
and core JS/TS in mixed packages, cross-framework imports, tests/coverage, quality policy,
provider-derived focused environment projections, and generation. Public repository/root/exact-target
env orchestration remains in M25.
The v1 framework set is closed at Vue, Svelte, Astro, and MDX under
[O43](../open-decisions.md); no additional framework is assigned to this
milestone. If a later scope decision assigns one, split delivery into explicit
predecessor milestones; do not hide additional adapters inside mixed-framework
testing.

## Contract References

- [Generation](../generation/), [quality](../quality/), [environments](../environments/), [tools](../tools/), [support matrix](../product/support-matrix.md), [testing](../testing/), and open decisions [O42 and O43](../open-decisions.md).

## Deliverables

- MDX wrapper/provider normalization, Gazelle adapter, quality classification, test/coverage ownership,
  and provider-derived environment/IDE plan and focused projection.
- Mixed-framework conformance suite and shared boundary limited to mechanics proven by concrete adapters.
- Per-capability evidence for Vue, Svelte, Astro, and MDX only.

## Work Packages

1. Prove and implement the MDX parser/compiler, expression dependency, target, provider, and quality boundary.
2. Add cross-framework ownership, import, naming, package, test/coverage, focused environment,
   IDE, and quality fixtures for Vue, Svelte, Astro, and MDX with core JS/TS.
3. Run mixed platform, external-consumer, remote, stale-cleanup, and unused-adapter suites.

## Milestone-Specific Evidence

- Markdown prose is not treated as executable code; expression dependencies use authoritative parsing.
- Mixed graphs prove deterministic single ownership, complete framework coverage attribution for every
  applicable support-matrix cell, focused environment projection, and no generic fallback or eager
  unused framework work.

## Out Of Scope

- Formats outside the reviewed inventory, arbitrary framework registration, and supported status.

## Completion Report Additions

- Record MDX scope plus the mixed-framework matrix, shared mechanics, cross-framework limitations, and adapter-test cells.
