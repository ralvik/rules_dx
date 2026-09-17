# Rules-lint performance comparison

Status: provisional methodology, open results. No parity claim until numbers land.

## Context

Accepted: the frozen `aspect_rules_lint` v2.8.0 tool list is the minimum first-release quality baseline, per [First-Release Tool Baseline](tool-baseline.md). The project neither depends on nor forks that project, per [Tool Integrations](../quality/tool-integrations.md) and [ADR 0007](../decisions/0007-tool-integration-model.md). A fair external comparison is therefore possible.

Open: no performance evidence exists yet for the quality pipeline against that baseline. This document records the comparison methodology only. Results arrive as a checked-in report, not a gate, until stable.

## Methodology

Provisional. Seed host only (`ubuntu-latest`, local-only) for fairness.

The harness uses a synthetic large tree with many targets and files in a mixed clean and dirty state. Benchmarks stay separate from examples: examples prove correctness as a consumer, while benchmarks need a controlled tree with repeated runs.

Metrics cover quality-only work matching the `rules_lint` surface: cold and warm wall time, Bazel action count via `aquery`, cache-hit rate, and peak memory where cheap to collect.

Fairness pins record the same tool versions where surfaces overlap, the same Bazel version from `.bazelversion` (currently `9.2.0`), and the same machine class. The report records the `rules_lint` pin (`v2.8.0` baseline) and any adapter-version skew explicitly.

## Current status

Open. No comparison numbers are recorded here. Gating on "faster than `rules_lint`" is explicitly rejected until noise is understood; numbers publish first, gates follow only if warranted.

Harness seed (issue #86 slice 2): `bazel run //perf:rules_lint_comparison` generates the deterministic synthetic tree (default 200 files, 10% dirty, seed 86) and emits a fairness-pinned JSON skeleton, proven by `bazel test //perf:rules_lint_comparison_test` (9/9: determinism, dirty mix + triggers, pins, report-not-gate). Seed-host run: 200 files / 20 dirty, tree `0a9fbc0f…8283de`, Bazel 9.2.0, `rules_lint` pin v2.8.0, host `linux_x86_64`. Measured quality-only runs and the checked-in results report follow next; no parity claim until then.

Owning tracker: [issue #86](https://github.com/ralvik/rules_dx/issues/86). Related: [ADR 0007](../decisions/0007-tool-integration-model.md), [First-Release Tool Baseline](tool-baseline.md), [Tool Integrations](../quality/tool-integrations.md), [issue #6](https://github.com/ralvik/rules_dx/issues/6) (quality registry), [issue #12](https://github.com/ralvik/rules_dx/issues/12) (dogfood).

Compatibility: benchmark and documentation only. No product change.
