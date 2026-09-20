#!/usr/bin/env bash
# Verification foundations harness (issues #86, #15, #12; relates #54).
#
# Stage 5 close-out (#54) runs the full battery on a clean tree, but three
# verification layers stay methodology-only with honest gaps:
# - #86 perf: synthetic-tree harness + fairness pins + report-not-gate
#   results exist; rules_lint-side numbers, full comparison report, and
#   any gate stay open (no parity claim until measured runs land).
# - #15 corpus: per-type split (corpus_markdown/corpus_starlark/corpus_toml,
#   plus corpus_json where preserved) landed with generation
#   (`dx generate`, Gazelle owns every corpus block, shared `tags =
#   ["corpus"]`); CI scopes query `attr(tags, corpus, ...)`, never
#   hand-maintained singletons.
# - #12 dogfood lane A: production code rides normal targets with
#   QualitySourcesInfo (pinned by //tools/ci:wrapper_sources); workspace-level
#   native policy (`//:ruff_config`, `//:biome_config`, `//:rustfmt_config`)
#   binds normal targets via direct `aspect_hints` through the shared
#   forwarder (pinned by //tools/ci:wrapper_sources); CI `dx lint`/`format`
#   scope covers the proven language trees (`//python/...`,
#   `//javascript/...`, `//rust/tests/fixtures/hello/...`, enforcing at --fail-on warning)
#   alongside the corpus. Remaining gaps stay open with no false claim:
#   broader trees (for example `//cli/...` BUILD docstring warnings),
#   CrateInfo/DepInfo context plumbing for Rust dep-coupled classes, and
#   vacuous classification-only families (Go/Java/Kotlin/Scala/C#/...).
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (12 checks) and records the gaps instead of claiming them.
#
# Versioned here, run by CI via `bazel run //tools/ci:verify_perf_corpus`,
# following //tools/ci:coverage_spill.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# #86: synthetic-tree harness exists and is deterministic.
if [[ -f "perf/rules_lint_comparison.sh" ]] &&
  grep -q -F -e 'tree_sha256' perf/rules_lint_comparison.sh; then
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
if grep -q -F -e '.bazelversion' perf/rules_lint_comparison.sh &&
  [[ -f ".bazelversion" ]]; then
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

# #86: harness + results provenance tests stay wired (no silent drift).
if grep -q -F -e 'rules_lint_comparison_test' perf/BUILD.bazel &&
  grep -q -F -e 'rules_lint_results_test' perf/BUILD.bazel; then
  ok
else
  bad "perf BUILD lost the comparison/results provenance tests"
fi

# #15: corpus split landed with generation — CI scopes query the shared
# `corpus` tag (per-type splits), never the legacy single `corpus` name;
# every corpus block is Gazelle-owned via `dx generate`.
if grep -q -F -e "attr(tags, corpus," .github/workflows/ci.yml &&
  ! grep -q -F -e "attr(name, '^corpus" .github/workflows/ci.yml &&
  grep -rln -F -e 'corpus_markdown' --include='BUILD.bazel' . 2>/dev/null | grep -q .; then
  ok
else
  bad "ci.yml lost the tag-scoped corpus split query (want attr(tags, corpus) + generated corpus_markdown, no legacy single-corpus query)"
fi

# #15: generation freshness still gates dogfood before quality converges.
if grep -q -F -e 'generate --check' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the generate --check freshness gate"
fi

# #12: ownership audits stay versioned (corpus for target-less, code for
# normal targets) alongside the wrapper-sources pin.
if [[ -f "tools/ci/corpus_audit.sh" && -f "tools/ci/code_ownership.sh" &&
  -f "tools/ci/wrapper_sources.sh" ]]; then
  ok
else
  bad "ownership harnesses missing (corpus_audit/code_ownership/wrapper_sources)"
fi

# #12 lane A: workspace-level native policy binds normal targets (root
# ruff/biome/rustfmt configs plus direct aspect_hints on the proof
# bindings); the shared forwarder carries hints to the QualitySourcesInfo
# owner.
if [[ -f "ruff.toml" && -f "biome.json" && -f "rustfmt.toml" ]] &&
  grep -q -F -e 'ruff_config' BUILD.bazel &&
  grep -q -F -e 'biome_config' BUILD.bazel &&
  grep -q -F -e 'aspect_hints' python/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints' javascript/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints' rust/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints' libs/starlark/wrapper.bzl; then
  ok
else
  bad "lane-A workspace-level native policy binding missing (root configs + proof aspect_hints + forwarder plumbing)"
fi

# #12 lane A: CI dx lint/format scope covers the proven language trees
# alongside the corpus (enforcing at --fail-on warning once clean).
if grep -q -F -e '//python/...' .github/workflows/ci.yml &&
  grep -q -F -e '//javascript/...' .github/workflows/ci.yml &&
  grep -q -F -e '//rust/tests/fixtures/hello/...' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the lane-A language-tree lint/format scope (want //python/... //javascript/... //rust/tests/fixtures/hello/... alongside corpus)"
fi

# Battery stays explicit-invocation for E2E (no wildcard-suite leakage).
if grep -q -F -e 'manual' tools/ci/BUILD.bazel; then
  ok
else
  bad "tools/ci BUILD lost the manual-tag E2E battery record"
fi

# Perf gate scope stays honest (issue #424): the single dx_startup gate is
# sufficient; cold/warm plus zero-unused-work per ADR 0014 consequences stay
# advisory with only dx_startup carrying gate:true.
if grep -q -F -e 'single `dx_startup` gate is sufficient' perf/README.md &&
  grep -q -F -e 'ADR 0014' perf/README.md &&
  grep -q -F -e 'gate:true' perf/baseline.json &&
  python3 -c "import json; d=json.load(open('perf/baseline.json'))['benchmarks']; gates=[k for k,v in d.items() if v.get('gate') is True]; assert gates==['dx_startup'], gates"; then
  ok
else
  bad "perf lost its single-gate scope record (want perf/README single dx_startup sufficient plus ADR 0014 plus baseline gate:true only dx_startup, issue #424)"
fi

dx_test_summary "verify perf corpus harness"
