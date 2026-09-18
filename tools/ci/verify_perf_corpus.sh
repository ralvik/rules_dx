#!/usr/bin/env bash
# Verification foundations harness (issues #86, #15, #12; relates #54).
#
# Stage 5 close-out (#54) runs the full battery on a clean tree, but three
# verification layers stay methodology-only with honest gaps:
# - #86 perf: synthetic-tree harness + fairness pins + report-not-gate
#   results exist; rules_lint-side numbers, full comparison report, and
#   any gate stay open (no parity claim until measured runs land).
# - #15 corpus: single `corpus` per directory stays the rule; the
#   per-type split (corpus_markdown/corpus_starlark/...) lands only with
#   generation (`dx generate`), never hand-maintained. CI scopes still
#   query `attr(name, '^corpus$')`.
# - #12 dogfood lane A: production code rides normal targets with
#   QualitySourcesInfo (pinned by //tools/ci:wrapper_sources); native
#   tool-config binding for own-tree runs plus CI `dx lint` scope
#   extension stay open.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (12 checks) and records the gaps instead of claiming them.
#
# Versioned here, run by CI via `bazel run //tools/ci:verify_perf_corpus`,
# following //tools/ci:coverage_spill.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# #86: synthetic-tree harness exists and is deterministic.
if [[ -f "perf/rules_lint_comparison.sh" ]] \
  && grep -q -F -e 'tree_sha256' perf/rules_lint_comparison.sh; then
  ok
else
  bad "perf synthetic-tree harness missing or lost determinism (tree_sha256)"
fi

# #86: fairness pin — frozen rules_lint v2.8.0 baseline recorded in code.
if grep -q -F -e 'v2.8.0' perf/rules_lint_comparison.sh; then
  ok
else
  bad "perf harness lost the frozen rules_lint v2.8.0 pin"
fi

# #86: fairness pin — same Bazel version from .bazelversion.
if grep -q -F -e '.bazelversion' perf/rules_lint_comparison.sh \
  && [[ -f ".bazelversion" ]]; then
  ok
else
  bad "perf harness lost the .bazelversion fairness pin"
fi

# #86: results stay report-not-gate with the frozen pin (no parity claim).
if python3 -c "import json,sys; d=json.load(open('perf/rules_lint_results.json')); sys.exit(0 if (d.get('gate') is False and d.get('rules_lint_pin')=='v2.8.0') else 1)"; then
  ok
else
  bad "perf results lost report-not-gate status or the v2.8.0 pin"
fi

# #86: methodology doc owns the open results, links the tracker, claims nothing.
if grep -q -F -e 'issues/86' docs/tools/rules_lint-comparison.md \
  && grep -q -F -e 'no parity claim' docs/tools/rules_lint-comparison.md; then
  ok
else
  bad "rules_lint-comparison doc lost its #86 link or no-parity-claim record"
fi

# #86: harness + results provenance tests stay wired (no silent drift).
if grep -q -F -e 'rules_lint_comparison_test' perf/BUILD.bazel \
  && grep -q -F -e 'rules_lint_results_test' perf/BUILD.bazel; then
  ok
else
  bad "perf BUILD lost the comparison/results provenance tests"
fi

# #15: no hand-split corpus — CI scopes still query the single `corpus`
# name; the per-type split lands only with generation.
if grep -q -F -e "attr(name, '^corpus" .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the single-corpus scope query (hand-split without generation?)"
fi

# #15: generation freshness still gates dogfood before quality converges.
if grep -q -F -e 'generate --check' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the generate --check freshness gate"
fi

# #15/#12/#86/#54: battery page still owns each layer (no silent promotion).
if grep -q -F -e 'issues/15' docs/testing/verification-matrix.md \
  && grep -q -F -e 'issues/86' docs/testing/verification-matrix.md \
  && grep -q -F -e 'issues/12' docs/testing/verification-matrix.md \
  && grep -q -F -e 'issues/54' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #15/#86/#12/#54 owner links"
fi

# #12: ownership audits stay versioned (corpus for target-less, code for
# normal targets) alongside the wrapper-sources pin.
if [[ -f "tools/ci/corpus_audit.sh" && -f "tools/ci/code_ownership.sh" \
  && -f "tools/ci/wrapper_sources.sh" ]]; then
  ok
else
  bad "ownership harnesses missing (corpus_audit/code_ownership/wrapper_sources)"
fi

# #12: matrix records lane A scope honestly (corpus + code ownership).
if grep -q -F -e 'lane A' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost the #12 lane A scope record"
fi

# Battery stays explicit-invocation for E2E (no wildcard-suite leakage).
if grep -q -F -e 'manual' tools/ci/BUILD.bazel; then
  ok
else
  bad "tools/ci BUILD lost the manual-tag E2E battery record"
fi

echo "verify perf corpus harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
