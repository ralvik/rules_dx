# M16: JavaScript And TypeScript Application Foundation

## Outcome

JavaScript and TypeScript applications generate, build, test, cover, and produce provider-derived
managed Node environment plans and focused projections.

## Scope

Add pinned `aspect_rules_js`, `aspect_rules_ts`, and `aspect_rules_jest`; narrow wrappers; first-party
JavaScript/TypeScript Gazelle extensions; strict pnpm importer scope; entries; version behavior; and managed
root and importer-local `node_modules` plans/projections for focused language targets. Public
repository/root/exact-target env planning, collection, and orchestration remain in M25.

## Contract References

- [JS/TS foundation decision](../decisions/0013-rust-javascript-typescript-foundations.md), [toolchain versions](../decisions/0012-language-toolchain-versions.md), [generation](../generation/), [environments](../environments/), [testing](../testing/), and open decision [O27](../open-decisions.md).

## Deliverables

- JS/TS library, binary, test, and project wrappers with Jest and coverage.
- Strict first-party extensions and provider-derived Node environment plans.

## Work Packages

1. Prove upstream providers, pnpm importers/stores, Jest, TypeScript, and version mappings.
2. Implement supported extensions, one-source ownership, tests, entries, strict imports, merge, and stale cleanup.
3. Project authoritative package artifacts into managed root and importer-local `node_modules`.

## Milestone-Specific Evidence

- Source-only local graphs and authoritative pnpm graphs pass without package-manager invocation or sidecars.
- Required-platform focused-target fixtures prove module formats, dependency-scope isolation,
  Jest/coverage, provider-derived environment projections, and laziness without public env orchestration.

## Out Of Scope

- Framework containers, JS/TS quality tools, and broad setup orchestration.

## Completion Report Additions

- Record ruleset versions, source classes, generated shapes, importer semantics, test behavior, and Node environment layout.
