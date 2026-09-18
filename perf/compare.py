#!/usr/bin/env python3
"""Perf comparison machinery (issue #215): median-of-N vs baseline with a
tolerance band (relative + absolute floor against runner noise).

Reads JSON-lines benchmark results (files or stdin), groups by benchmark name,
computes the median, and compares against perf/baseline.json:

- Absolute gate: benchmarks with `gate: true` fail (exit 1) when
  median > budget_ms. Currently dx_startup only; a blown generous budget is
  unambiguous. All other budgets are recorded but advisory.
- Relative comparison: regression iff
  median > baseline*(1+tolerance) AND median-baseline > floor_ms.
  Regressions are advisory (warn-only): exit 0, print REGRESSION lines, and
  (in CI) post a warn-only PR comment. Relative comparison stays advisory
  until months of stable signal, then promote per-benchmark.

Usage:
  bazel run //perf:bench_micro > /tmp/micro.jsonl
  python3 perf/compare.py --baseline perf/baseline.json /tmp/micro.jsonl
  python3 perf/compare.py --baseline perf/baseline.json --summary "$GITHUB_STEP_SUMMARY" results.jsonl

Relative ``--baseline`` paths resolve against BUILD_WORKSPACE_DIRECTORY
when set (``bazel run``), otherwise against the current directory.

Output: human report on stdout; optional GitHub step-summary markdown via
--summary. Exit 1 only on gated absolute-budget breach or unparsable input;
never on relative regression (warn-only by design).
"""
import argparse
import json
import os
import pathlib
import statistics
import sys


def resolve_baseline(path):
    """Resolve ``--baseline`` against the real checkout.

    Under ``bazel run`` the process starts in the target's runfiles
    directory inside bazel-out (where a relative ``perf/baseline.json``
    does not exist), so a relative baseline must resolve against
    ``BUILD_WORKSPACE_DIRECTORY`` — the same convention as
    ``perf/bench.sh`` and ``perf/regenerate.py``. Absolute paths pass
    through unchanged; direct ``python3 perf/compare.py`` runs from the
    checkout already, where the relative path resolves naturally.
    """
    candidate = pathlib.Path(path)
    if candidate.is_absolute():
        return candidate
    workspace = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if workspace:
        return pathlib.Path(workspace) / candidate
    return candidate


def load_baseline(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def load_results(paths):
    samples = {}
    if not paths:
        for line in sys.stdin:
            line = line.strip()
            if line:
                record = json.loads(line)
                samples.setdefault(record["benchmark"], []).append(float(record["duration_ms"]))
        return samples
    for path in paths:
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)
                samples.setdefault(record["benchmark"], []).append(float(record["duration_ms"]))
    return samples


def median(values):
    return statistics.median(sorted(values))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", default="perf/baseline.json")
    parser.add_argument("--tolerance", type=float, default=None)
    parser.add_argument("--summary", default=None)
    parser.add_argument("results", nargs="*")
    args = parser.parse_args()

    try:
        baseline_doc = load_baseline(str(resolve_baseline(args.baseline)))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"perf compare: cannot read baseline {args.baseline}: {exc}")
        return 2
    try:
        samples = load_results(args.results)
    except (OSError, json.JSONDecodeError) as exc:
        print(f"perf compare: cannot read results: {exc}")
        return 2
    if not samples:
        print("perf compare: no benchmark samples found")
        return 2

    default_tolerance = float(baseline_doc.get("tolerance_relative_default", 0.25))
    benchmarks = baseline_doc.get("benchmarks", {})
    failures = []
    warnings = []
    lines = []
    for name in sorted(samples):
        values = samples[name]
        med = median(values)
        spec = benchmarks.get(name)
        if spec is None:
            warnings.append(f"UNKNOWN benchmark {name!r}: no baseline entry; ignoring")
            lines.append(f"UNKNOWN {name}: median {med:.1f}ms over {len(values)} runs (no baseline)")
            continue
        base = float(spec["baseline_ms"])
        budget = float(spec["budget_ms"])
        floor = float(spec.get("floor_ms", 0))
        tolerance = float(spec.get("tolerance_relative", args.tolerance if args.tolerance is not None else default_tolerance))
        gate = bool(spec.get("gate", False))
        lines.append(
            f"{name}: median {med:.1f}ms (baseline {base:.1f}ms, budget {budget:.0f}ms, "
            f"n={len(values)}, tolerance {tolerance:.0%} + floor {floor:.0f}ms)"
        )
        if med > budget:
            if gate:
                failures.append(f"GATE {name}: median {med:.1f}ms exceeds budget {budget:.0f}ms")
            else:
                warnings.append(
                    f"BUDGET-ADVISORY {name}: median {med:.1f}ms exceeds budget {budget:.0f}ms (advisory; only gated budgets fail)"
                )
        if med > base * (1 + tolerance) and (med - base) > floor:
            warnings.append(
                f"REGRESSION {name}: median {med:.1f}ms vs baseline {base:.1f}ms "
                f"(+{med - base:.1f}ms, tolerance {tolerance:.0%} + floor {floor:.0f}ms; advisory)"
            )

    print("perf compare report")
    for line in lines:
        print(f"  {line}")
    if not failures and not warnings:
        print("  No regressions; all medians within tolerance and budgets.")
    for warning in warnings:
        print(f"  WARNING: {warning}")
    for failure in failures:
        print(f"  FAIL: {failure}")

    if args.summary:
        with open(args.summary, "a", encoding="utf-8") as fh:
            fh.write("## Perf comparison\n\n")
            for line in lines:
                fh.write(f"- `{line}`\n")
            if failures:
                fh.write("\n")
                for failure in failures:
                    fh.write(f"- **{failure}**\n")
            if warnings:
                fh.write("\n")
                for warning in warnings:
                    fh.write(f"- {warning}\n")
            if not failures and not warnings:
                fh.write("\n- No regressions; all medians within tolerance and budgets.\n")

    if failures:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
