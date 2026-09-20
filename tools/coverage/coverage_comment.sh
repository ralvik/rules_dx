#!/usr/bin/env bash
# First-party coverage PR summary renderer.
#
# Renders the compact Codecov-style summary comment from the Bazel-owned
# gate verdict, never as a substitute for the gate. Source of truth stays
# `bazel coverage --combined_report=lcov` through `dx coverage` /
# `coverage_bin` (`docs/cli/standard-reports.md#lcov`,
# `tools/bazelrc/preset.bazelrc`, `tools/ci/coverage_cell.sh`,
# `tools/coverage/seed-inventory.txt`): this script only presents the
# verdict already computed by the gate.
#
# Contract (docs/github-ci.md#reporting, docs/testing/github-ci.md):
# one integration-owned updated summary comment per PR with the
# `dx-coverage-summary: <cell>` marker for dedup, revision/run identity,
# per-cell exact covered/eligible counts with uncovered file:line
# locations, completeness, and LCOV artifact links. Cells, languages, and
# metrics stay separate: no cross-cell union, no averaged percentages, no
# rounding up. Starlark behavioral fallback (if ever used) stays separate
# from line coverage. Findings outside the diff stay in counts only; no
# invented review locations. Diff-mapped review threads are opt-in and not
# emitted here.
#
# Failure semantics: missing reports, incomplete instrumentation,
# uninventoried sources, and uncovered non-ignored lines render as visible
# FAIL/INCOMPLETE. Rendering always exits 0 on valid inputs (even when the
# gate failed) so the comment can never turn a failing gate into success;
# the gate exit stays separate in CI. Publication failure (gh comment)
# fails CI separately in the workflow while preserving this output.
#
# Usage:
#   coverage_comment.sh --cell <cell> --revision <sha> --run <run-url-or-id>
#     --out <markdown> [--gate-output <verdict.txt>] [--gate-exit <0|1>]
#     [--lcov <path>] [--dx-output <dx-stderr.txt>] [--dx-exit <code>]
set -euo pipefail

cell=""
revision=""
run_ref=""
out=""
gate_output=""
gate_exit=""
lcov_path=""
dx_output=""
dx_exit=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cell) cell="${2:-}"; shift 2 ;;
    --revision) revision="${2:-}"; shift 2 ;;
    --run) run_ref="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    --gate-output) gate_output="${2:-}"; shift 2 ;;
    --gate-exit) gate_exit="${2:-}"; shift 2 ;;
    --lcov) lcov_path="${2:-}"; shift 2 ;;
    --dx-output) dx_output="${2:-}"; shift 2 ;;
    --dx-exit) dx_exit="${2:-}"; shift 2 ;;
    --help|-h)
      echo "usage: coverage_comment.sh --cell <cell> --revision <sha> --run <run> --out <md> [--gate-output <f> --gate-exit <0|1>] [--lcov <path>] [--dx-output <f> --dx-exit <code>]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      echo "usage: coverage_comment.sh --cell <cell> --revision <sha> --run <run> --out <md> [--gate-output <f> --gate-exit <0|1>] [--lcov <path>] [--dx-output <f> --dx-exit <code>]" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$cell" || -z "$revision" || -z "$run_ref" || -z "$out" ]]; then
  echo "missing required --cell/--revision/--run/--out" >&2
  exit 2
fi

verdict_line="coverage gate: INCOMPLETE (no gate verdict supplied)"
verdict_status="INCOMPLETE"
gate_body=""
uncovered_section="Uncovered locations: not available (no gate verdict)."
errors_section="Completeness: unknown (no gate verdict supplied)."
dx_section=""

if [[ -n "$gate_output" ]]; then
  if [[ -f "$gate_output" ]]; then
    gate_body="$(cat "$gate_output")"
    first_line="$(head -n 1 "$gate_output" || true)"
    if [[ "$first_line" == *"coverage gate: PASS"* ]]; then
      verdict_line="$first_line"
      verdict_status="PASS"
    elif [[ "$first_line" == *"coverage gate: FAIL"* ]]; then
      verdict_line="$first_line"
      verdict_status="FAIL"
    else
      verdict_line="coverage gate: FAIL (unrecognized verdict header)"
      verdict_status="FAIL"
    fi
    # Collect uncovered file:line locations (rendered as path:line).
    uncovered_hits="$(grep -o -E '[A-Za-z0-9_./+-]+\.[a-z]+:[0-9]+' "$gate_output" || true)"
    if [[ -n "$uncovered_hits" ]]; then
      uncovered_list="$(echo "$uncovered_hits" | LC_ALL=C sort -u | tr '\n' ' ')"
      uncovered_section="Uncovered locations: $uncovered_list"
    else
      if [[ "$verdict_status" == "PASS" ]]; then
        uncovered_section="Uncovered locations: none (zero uncovered lines)."
      else
        uncovered_section="Uncovered locations: see gate detail below (counts only; no invented review positions)."
      fi
    fi
    if grep -q -E '^errors:' "$gate_output"; then
      errors_section="Completeness: gate reports errors (missing reports, uninventoried sources, or absent eligible sources fail closed; see detail)."
    else
      errors_section="Completeness: gate reports no errors section."
    fi
  else
    verdict_line="coverage gate: FAIL (missing gate verdict file: $gate_output)"
    verdict_status="FAIL"
    gate_body="missing gate verdict file: $gate_output"
    uncovered_section="Uncovered locations: unknown (missing verdict)."
    errors_section="Completeness: incomplete (missing verdict file fails closed)."
  fi
fi

if [[ -n "$dx_output" && -f "$dx_output" ]]; then
  dx_line="$(grep -E -e 'coverage .*lines.*meets minimum|coverage_below_minimum' "$dx_output" | head -n 1 || true)"
  if [[ -n "$dx_line" ]]; then
    dx_section="Rate gate (dx coverage): $dx_line"
  else
    dx_section="Rate gate (dx coverage): exit ${dx_exit:-unknown}; see logs."
  fi
elif [[ -n "$dx_exit" ]]; then
  dx_section="Rate gate (dx coverage): exit $dx_exit."
fi

if [[ -z "$gate_exit" ]]; then
  gate_exit_note="gate exit: not supplied (comment is presentation only, never the gate)"
else
  gate_exit_note="gate exit: $gate_exit (comment never changes this outcome)"
fi

lcov_note="LCOV: ${lcov_path:-not supplied} (Bazel-owned combined report; see workflow run $run_ref)."
if [[ -n "$lcov_path" && ! -f "$lcov_path" ]]; then
  lcov_note="LCOV: $lcov_path (absent at render time; run $run_ref retains the authoritative copy or the missing-report failure)."
fi

{
  echo "<!-- dx-coverage-summary: $cell -->"
  echo "## Coverage summary — $cell — $verdict_status"
  echo ""
  echo "Analyzed revision: \`$revision\` | Run: $run_ref | Cell: \`$cell\` (seed host, local-only)."
  echo ""
  echo "$verdict_line"
  echo ""
  echo "$uncovered_section"
  echo ""
  echo "$errors_section"
  echo ""
  if [[ -n "$dx_section" ]]; then
    echo "$dx_section"
    echo ""
  fi
  echo "$gate_exit_note"
  echo ""
  echo "$lcov_note"
  echo ""
  echo "Scope: per-cell exact covered/eligible counts only; no cross-cell union, no averaged percentages, no rounding up. Languages and metrics stay separate. Starlark line data (if any) stays separate from the behavioral fallback per docs/testing/README.md#coverage."
  echo ""
  echo "Findings outside the diff stay in counts only; no invented review locations. Diff-mapped review threads are opt-in and not emitted by this summary."
  echo ""
  echo "Codecov stays opt-in only and is never required; this first-party comment is the adopted PR surface (docs/testing/README.md#github-coverage-reporting)."
  echo ""
  echo "<details><summary>Gate detail (exact counts decide)</summary>"
  echo ""
  echo '```'
  if [[ -n "$gate_body" ]]; then
    echo "$gate_body"
  else
    echo "(no gate verdict supplied)"
  fi
  echo '```'
  echo "</details>"
} > "$out.tmp"
mv "$out.tmp" "$out"
