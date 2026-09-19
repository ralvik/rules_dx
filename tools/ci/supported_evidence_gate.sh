#!/usr/bin/env bash
# Supported-evidence promotion gate (issue #301).
#
# No cell is Supported: promotion requires platform plus consumer plus
# release evidence per the support-matrix status lifecycle
# (Planned -> Seed-host-delivered -> Platform-qualified -> Supported).
# Seed-host evidence is delivered (corpus dogfood, Layer-2 matrix for the
# seed languages, generation freshness, adopt-* external-consumer proof,
# hermetic CLI-contract pins (issue #407 replaces nested E2E),
# required-core depcheck fixtures, `dx update` live execution,
# perf report-not-gate, seed-only coverage/remote qualification under #308);
# audit live execution, docs-pipeline, env/codegen, non-seed platform cells
# (issue #298), admitted depcheck
# expansion, and release evidence (SBOM/provenance, signing/attestations,
# BCR submission, GHCR route, consumer verification) remain open gaps
# tracked in their owning issues.
# This harness machine-checks the gate statically on a clean tree:
# no Supported status cell exists in either matrix, every Delivered claim
# has a backing harness/workspace, and no Open gap is falsely claimed as
# landed. Functional checks only (table-cell validation, file existence,
# workflow/config pins, code symbols), no tracker-sentence greps.
#
# Versioned here, run by CI via `bazel run //tools/ci:supported_evidence_gate`.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Module stays unpublishable: no release cut, no Supported promotion.
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted from unpublishable 0.0.0 (no release cut per #301 gate)"
fi

# No-publish invariant: no version tags, no committed dist/release outputs.
if [[ -z "$(git tag --list 'v*' | head -1)" ]] &&
  [[ -z "$(git ls-files 'dist/*' 'release/*' 2>/dev/null | head -1)" ]]; then
  ok
else
  bad "a version tag or committed dist/release output appeared without release evidence"
fi

# Support-matrix lifecycle owns promotion (decided contract).
if grep -q -F -e '`Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`' docs/product/support-matrix.md &&
  grep -q -F -e 'Status cells are owned by' docs/product/support-matrix.md &&
  grep -q -F -e 'promotion to `Supported` requires release evidence' docs/product/support-matrix.md; then
  ok
else
  bad "support-matrix lost its status-lifecycle promotion contract"
fi

# Support-matrix records seed-delivered vs unqualified (platform evidence).
if grep -q -F -e 'Seed-host-delivered' docs/product/support-matrix.md &&
  grep -q -F -e 'Unqualified: clean `unsupported_platform` refusal' docs/product/support-matrix.md; then
  ok
else
  bad "support-matrix lost its seed-delivered vs unqualified platform record"
fi

# No Supported status cell in support-matrix tables (promotion gate core).
# Table rows start with `|`; prose mentions of Supported are allowed.
if ! grep -E -e '^\|.*\| *`?Supported`? *\|' docs/product/support-matrix.md | grep -q .; then
  ok
else
  bad "a Supported status cell appeared in support-matrix tables without evidence"
fi

# No Supported status cell in verification-matrix tables.
if ! grep -E -e '^\|.*\| *`?Supported`? *\|' docs/testing/verification-matrix.md | grep -q .; then
  ok
else
  bad "a Supported status cell appeared in verification-matrix tables without evidence"
fi

# Verification-matrix vocabulary stays Delivered/Open/Planning only/Tracked
# (no silent new status that implies support). Audit plus update are Delivered.
if grep -q -F -e '| Rust | Delivered' docs/testing/verification-matrix.md &&
  grep -q -F -e '| Go | Delivered (code ownership) | Open (adapter-less)' docs/testing/verification-matrix.md &&
  grep -q -F -e '| Delivered | Open | Open |' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix status vocabulary drifted (want Delivered/Open/Planning only/Tracked with audit/update Delivered)"
fi

# Corpus dogfood Delivered has backing harnesses + CI wiring.
if [[ -f "tools/ci/corpus_audit.sh" ]] &&
  [[ -f "tools/ci/code_ownership.sh" ]] &&
  grep -q -F -e '//tools/ci:corpus_audit' .github/workflows/ci.yml &&
  grep -q -F -e '//tools/ci:code_ownership' .github/workflows/ci.yml; then
  ok
else
  bad "corpus-dogfood Delivered lacks backing harnesses or CI wiring"
fi

# Layer-2 runner matrix Delivered (seed languages) has runner-matrix docs.
if [[ -f "docs/quality/runner-matrix.md" ]] &&
  [[ -f "quality/pipeline_tests.bzl" || -f "quality/parity_tests.bzl" ]]; then
  ok
else
  bad "Layer-2 matrix Delivered lacks runner-matrix docs or pipeline tests"
fi

# Generation Delivered has freshness gate + generation docs.
if grep -q -F -e 'generate --check //...' .github/workflows/ci.yml &&
  [[ -f "docs/testing/generation.md" ]]; then
  ok
else
  bad "generation Delivered lacks generate --check gate or generation docs"
fi

# Examples Delivered (adopt-*) has workspaces + laziness proof harnesses.
if [[ -d "examples/adopt-rust" ]] &&
  [[ -d "examples/adopt-python" ]] &&
  [[ -d "examples/adopt-js-ts" ]] &&
  [[ -f "tools/ci/examples_readme.sh" ]] &&
  [[ -f "tools/ci/examples_laziness.sh" ]] &&
  [[ -f "tools/ci/examples_laziness_runtime.sh" ]]; then
  ok
else
  bad "examples Delivered lacks adopt-* workspaces or laziness proof harnesses"
fi

# CLI-contract Delivered (issue #407, replaces nested E2E): hermetic pins
# under `bazel test //...` plus the adopt-rust dx_dev smoke in normal CI.
# What is lost (real-daemon exit 3, real Buildifier rewrite, full consumer
# wiring now smoke-only) is recorded in docs/testing/verification-matrix.md.
if [[ ! -d "integration" ]] &&
  [[ ! -f "tools/ci/e2e.sh" ]] &&
  [[ -f "cli/cli/src/exec/test_support.rs" ]] &&
  grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' .github/workflows/ci.yml &&
  grep -q -F -e 'real-daemon exit 3' docs/testing/verification-matrix.md; then
  ok
else
  bad "CLI-contract Delivered lacks hermetic pins + adopt-rust smoke + loss record (issue #407)"
fi

# Perf Tracked stays report-not-gate with fairness pin.
if [[ -f "perf/compare.py" ]] &&
  grep -q -F -e '"rules_lint_pin": "v2.8.0"' perf/rules_lint_results.json; then
  ok
else
  bad "perf Tracked lost its comparator or v2.8.0 fairness pin"
fi

# Depcheck Delivered for required core: contract plus fixtures plus targets.
if [[ -f "tools/ci/depcheck_contract.sh" ]] &&
  [[ -f "tools/depcheck/depcheck.py" ]] &&
  [[ -f "tools/depcheck/BUILD.bazel" ]]; then
  ok
else
  bad "depcheck Delivered lost its contract, checker, or targets"
fi

# Audit plus update Delivered: audit live execution, update live execution, plus guards harness.
if grep -q -F -e 'CODE_AUDIT_FAILED' cli/cli/src/exec/common.rs &&
  grep -q -F -e 'CODE_UPDATE_FAILED' cli/cli/src/exec/common.rs &&
  grep -q -F -e 'dx_update::backend::plan' cli/cli/src/exec/update.rs &&
  grep -q -F -e 'dx_audit::backend::plan_secrets' cli/cli/src/exec/audit.rs &&
  [[ -f "tools/ci/audit_update_guards.sh" ]]; then
  ok
else
  bad "audit/update Delivered lost live codes or guards harness"
fi

# Coverage seed-only qualified (#308): cell gate + versioned inventory and
# registry plus qualification harness, no cross-cell union.
if [[ -f "tools/ci/coverage_cell.sh" ]] &&
  [[ -f "tools/coverage/seed-inventory.txt" ]] &&
  [[ -f "tools/coverage/cells.txt" ]] &&
  [[ -f "tools/ci/coverage_qualification.sh" ]] &&
  grep -q -F -e 'seed cell' tools/coverage/seed-inventory.txt; then
  ok
else
  bad "coverage seed-only lost its cell gate, inventory, registry, or qualification harness"
fi

# Env/codegen + docs-pipeline Open: backlog contracts pin frozen halves.
if [[ -f "tools/ci/backlog_contracts.sh" ]]; then
  ok
else
  bad "env/codegen/docs-pipeline Open lost its backlog-contracts harness"
fi

# Release evidence Open: trust + closeout harnesses, no BCR tooling in workflows.
if [[ -f "tools/ci/publish_trust.sh" ]] &&
  [[ -f "tools/ci/distribution_closeout_guards.sh" ]] &&
  ! grep -rn -E -e 'publish-to-bcr|bcr publish' .github/ 2>/dev/null | grep -v -E -e '^\s*#' | grep -q .; then
  ok
else
  bad "release Open lost trust/closeout harnesses or gained BCR tooling"
fi

# Consumer verification Open but self-call smoke present (build-only seed).
if [[ -f ".github/workflows/reusable-consumer.yml" ]] &&
  grep -q -F -e 'rules_dx_version: "0.0.0"' examples/consumer-ci/caller.yml; then
  ok
else
  bad "consumer Open lost reusable workflow or 0.0.0 self-call pin"
fi

# Testing README acceptance evidence blocks Supported without full cells.
if grep -q -F -e '**Supported** evidence links every required adapter' docs/testing/README.md &&
  grep -q -F -e 'A missing cell remains an explicit gap and blocks that support' docs/testing/README.md; then
  ok
else
  bad "testing README lost its Supported-evidence blocking contract"
fi

dx_test_summary "supported evidence gate harness"
