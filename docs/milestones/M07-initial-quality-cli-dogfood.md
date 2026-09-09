# M07: Initial Quality CLI And Full Initial Dogfood

## Outcome

`dx lint`, `dx typecheck`, and `dx format` run the initial adapters, and this repository uses the full path in CI.

## Scope

Wire quality command plans, check/default modes, thresholds, standard reports, bounded virtual convergence,
and no-scope repository operation for the M04 adapters. Typecheck is a valid no-op where no adapter applies.

## Contract References

- [CLI command surface](../decisions/0006-cli-command-surface.md), [CLI](../cli/), [quality](../quality/), and [testing](../testing/).

## Deliverables

- Initial quality command registry, policy projection, reports, and dogfood CI workflows.
- Stable stage ordering and ten-round convergence enforcement.

## Work Packages

1. Plan quality invocations and canonical workspace policy selection.
2. Project normalized results into text, diff, NDJSON, and standard reports.
3. Switch repository dogfood to `dx` and verify direct-Bazel semantic parity.

## Milestone-Specific Evidence

- Check/default and direct-Bazel/CLI runs agree on findings, changes, thresholds, and status.
- Full corpus dogfood passes with deterministic no-op and convergence behavior.

## Out Of Scope

- Path/label resolution, generation, additional language foundations, and support promotion.
- Reusable consumer GitHub CI delivery belongs to [M27](M27-consumer-ci.md), not this
  repository-only initial dogfood slice.

## Completion Report Additions

- Record command modes, dogfood invocation, selected stage order, convergence cases, and parity deviations.
