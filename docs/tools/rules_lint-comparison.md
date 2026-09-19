# Rules-lint performance comparison

Status: provisional methodology, reported (report-not-gate). No parity claim.

## Context

Accepted: the frozen `aspect_rules_lint` v2.8.0 tool list is the minimum first-release quality baseline, per [First-Release Tool Baseline](tool-baseline.md). The project neither depends on nor forks that project, per [Tool Integrations](../quality/tool-integrations.md) and [ADR 0007](../decisions/0007-tool-integration-model.md). A fair external comparison is therefore possible.

Reported: seed-host quality-only evidence is checked in as `perf/rules_lint_results.json` (report-not-gate, no parity claim). This document records the comparison methodology plus the checked-in report. Gates follow only if/when noise is understood.

## Methodology

Provisional. Seed host only (`ubuntu-latest`, local-only) for fairness.

The harness uses a synthetic large tree with many targets and files in a mixed clean and dirty state. Benchmarks stay separate from examples: examples prove correctness as a consumer, while benchmarks need a controlled tree with repeated runs.

Metrics cover quality-only work matching the `rules_lint` surface: cold and warm wall time, Bazel action count via `aquery`, cache-hit rate, and peak memory where cheap to collect.

Fairness pins record the same tool versions where surfaces overlap, the same Bazel version from `.bazelversion` (currently `9.2.0`), and the same machine class. The report records the `rules_lint` pin (`v2.8.0` baseline) and any adapter-version skew explicitly.

## Current status

Reported. Comparison numbers are recorded in `perf/rules_lint_results.json` (report-not-gate, no parity claim). Gating on "faster than `rules_lint`" is explicitly rejected until noise is understood; numbers publish first, gates follow only if warranted.

Harness seed (open work slice 2): `bazel run //perf:rules_lint_comparison` generates the deterministic synthetic tree (default 200 files, 10% dirty, seed 86) and emits a fairness-pinned JSON skeleton, proven by `bazel test //perf:rules_lint_comparison_test` (9/9: determinism, dirty mix + triggers, pins, report-not-gate). Seed-host run: 200 files / 20 dirty, tree `0a9fbc0f…8283de`, Bazel 9.2.0, `rules_lint` pin v2.8.0, host `linux_x86_64`. Measured quality-only runs and the checked-in results report follow next; no parity claim until then.

Measured seed-host slice (open work slice 3): `perf/rules_lint_results.json` records server-warm quality-only numbers (report-not-gate, no parity claim), proven by `bazel test //perf:rules_lint_results_test` (14/14: pins, quality-sample aquery + timing, runner timing, regen digest match). Synth tree 200 files / 20 dirty, tree `0a9fbc0f…8283de`, generation ~79ms; quality sample `//perf:corpus` lint+format yields 2 actions (`DxRealQualityLint`, `DxRealQualityFormat`), server-warm no-cache ~379ms vs warm ~329ms (Bazel overhead dominates the 2-action sample); runner test `//quality/runner:quality_runner_test` aquery 6 actions, cold-ish (with `--nocache_test_results`) ~1812ms executing 1/1 vs warm ~990ms executing 0/1 (cache-hit evidence); peak memory explicitly uncollected (gap, not claimed). Post-shutdown cold measured next; `rules_lint`-side numbers follow after.

Post-shutdown cold slice (open work slice 4): quality sample `//perf:corpus` aspect build post-`bazel shutdown` records ~6419ms wall (~6356ms Bazel elapsed, fresh 9.2.0 server startup + analysis of 652 packages / 28179 targets / 114 aspects, 1615 disk-cache hits) vs immediate warm rebuild ~249ms (0 packages loaded, 1 internal, 0.231s elapsed). The cold-vs-warm delta is server startup plus analysis, not action execution — the 2-action sample stays cache-hit throughout. Single seed-host run, report-not-gate, no parity claim; proven by `bazel test //perf:rules_lint_results_test` (28/28: pins including exact Bazel 9.2.0 + dirty_pct 10 + dirty consistency + aquery-mnemonics consistency + post-shutdown elapsed consistency + benchmark-identity measured + no-cache flag explicit + runner cache-hit ordering + executed + warm-after-cold evidence, quality-sample aquery + server-warm/no-cache/post-shutdown timings + evidence, runner timing, seed generation_ms positive + 64-hex tree digest, regen digest match). Remaining: `rules_lint`-side numbers plus the full checked-in comparison report.

Full comparison report (closing slice): `perf/rules_lint_results.json` now carries explicit `fairness` pins (Bazel 9.2.0 from `.bazelversion`, host `linux_x86_64`, machine class `ubuntu-latest, local-only, no remote`, `rules_lint` pin `v2.8.0`, dx tool versions for biome 2.5.12 / buildifier 8.5.1 / ruff 0.16.7 / taplo 0.10.0 / ty 0.0.80 / vale 3.20.0 / eslint 10.10.0 / prettier 3.9.6 / tsc 5.9.3 / flake8 7.3.0 / pydoclint 0.9.1 / pylint 4.0.8 / rustfmt 1.98.0, plus explicit adapter-version skew noting independent dx currency per ADR 0007 with no parity claim) and a `comparison` section (dx-side measured summary, `rules_lint`-side reference methodology requiring the same synthetic tree, same Bazel, same host for any future direct run, with direct execution excluded per ADR 0007 no-dep/no-fork, plus methodology and report-not-gate verdict). Proven by `bazel test //perf:rules_lint_results_test` (40/40: prior 28 plus fairness-pin consistency, dx tool versions, skew, same-tool policy, dx-side/comparison/methodology/verdict checks). Report-not-gate, no parity claim.

Owning tracker: open work. Related: [ADR 0007](../decisions/0007-tool-integration-model.md), [First-Release Tool Baseline](tool-baseline.md), [Tool Integrations](../quality/tool-integrations.md), open work (quality registry), open work (dogfood).

Compatibility: benchmark and documentation only. No product change.
