# M17: JavaScript And TypeScript Quality

## Outcome

JavaScript and TypeScript quality is adapter-tested with Biome defaults, ESLint and Prettier alternatives, and `tsc` typechecking.

## Scope

Implement Biome lint/format, ESLint lint, Prettier format, and target-coupled `tsc` diagnostics with
private Node acquisition, native configs, semantic-class subsets, and measured formatter composition.

## Contract References

- [Quality](../quality/), [tools](../tools/), [generation](../generation/), [testing](../testing/), and open decision [O28](../open-decisions.md).

## Deliverables

- JS, JSX, TypeScript, TSX, and JSON adapter manifests and config bindings.
- Private pure-JavaScript tool graph and shared managed Node runtime.

## Work Packages

1. Prove Biome artifacts, authoritative `tsc`, and private Node ESLint/Prettier acquisition.
2. Implement diagnostics, fixes, formatting, native configs, and source subsets.
3. Benchmark Biome/Prettier ordering and certify convergence, cycles, and iteration limits.

## Milestone-Specific Evidence

- Consumers use no tool package manifest, install/lifecycle script, native addon build, or ambient Node.
- `tsc` uses `typescript_project` context; formatter order is independent of user/result order.

## Out Of Scope

- Prettier plugins outside framework needs, framework build adapters, and supported status.

## Completion Report Additions

- Record defaults/alternatives, package graph, runtime identity, `tsc` mapping, measured stage order, and adapter-test matrix.
