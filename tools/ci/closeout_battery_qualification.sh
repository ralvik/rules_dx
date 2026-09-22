#!/usr/bin/env bash
# Close-out battery plus docs-gate qualification harness.
#
# Qualifies the as-built close-out battery plus docs gate with fixture
# evidence and owned gaps, without claiming Supported or full-tree green
# here:
# - delivered: battery commands pinned in `.github/workflows/ci.yml`
#   (build plus test plus coverage plus prove plus dogfood-freshness plus
#   devcontainer-check plus docs-ci plus dogfood, with per-host
#   build/test/coverage jobs) plus the docs gate (docs-ci self-call over
#   `//docs/...` via the reusable-docs check-only contract);
# - wiring: BUILD target, dogfood-freshness step, Battery accepted record
#   with full-tree green owned by CI via this harness;
# - open owned gaps: platform plus consumer plus release evidence, full
#   rebuild green owned by CI jobs (not re-claimed here), exact per-host
#   green beyond static pins, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:closeout_battery_qualification`,
# following //tools/ci:ci_matrix_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

testing_readme="docs/testing/strategy-details.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
reusable=".github/workflows/reusable-docs.yml"

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
if [[ ! -f "docs/roadmap.md" ]]; then
  ok
else
  bad "docs/roadmap.md still exists (planned work lives in GitHub issues only, #981)"
fi

# Status lives in the short support matrix; per-cell dumps are deleted under #988.
if [[ ! -f "docs/testing/verification-matrix.md" ]] &&
  [[ ! -f "docs/testing/verification-matrix-remaining.md" ]] &&
  [[ ! -f "docs/product/promotion-checklist.md" ]]; then
  ok
else
  bad "status dumps still exist (merged into docs/product/support-matrix.md under #988)"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "closeout_battery_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the closeout_battery_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:closeout_battery_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the closeout_battery_qualification step (want dogfood-freshness)"
fi

# Build battery: full build plus the adopt-rust dx_dev smoke.
if grep -q -F -e 'bazel build --noshow_progress //...' "$ci" &&
  grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' "$ci"; then
  ok
else
  bad "build battery lost (want bazel build //... plus dx_dev smoke in ci.yml)"
fi

# Test battery: full test with hermetic CLI-contract pins, no manual.
# Tuned bounded flaky retries plus per-test timeout cap.
if grep -q -F -e 'bazel test --noshow_progress --flaky_test_attempts=3 --test_timeout=300 //...' "$ci"; then
  ok
else
  bad "test battery lost (want bazel test //... in ci.yml)"
fi

# Coverage battery: seed gate plus report guards.
if grep -q -F -e 'coverage --min-coverage 97 //...' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_report_guards' "$ci"; then
  ok
else
  bad "coverage battery lost (want dx coverage gate plus report guards in ci.yml)"
fi

# Prove battery: the twelve prove harnesses stay wired.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:target_tags' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_cell' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_spill' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:musl_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:macos_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:ci_matrix_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_hygiene' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_policy' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:publish_trust' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_contract' "$ci"; then
  ok
else
  bad "prove battery lost (want twelve prove harnesses in ci.yml)"
fi

# Dogfood core: freshness plus ownership audits.
if grep -q -F -e 'generate --check //...' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:corpus_audit' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:code_ownership' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:non_dogfed_paths' "$ci"; then
  ok
else
  bad "dogfood core lost (want generate --check plus corpus/code/non-dogfed in ci.yml)"
fi

# Dogfood extended: qualification sweep stays wired.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:supported_evidence_gate' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:quality_adapters_parity' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:env_codegen_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:docs_pipeline_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:consumer_ci_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:file_family_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:helper_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:clap_tokenizer_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:hello_smoke_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:parser_sample_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cli_contract_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:musl_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:macos_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:ci_matrix_qualification' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:closeout_battery_qualification' "$ci"; then
  ok
else
  bad "dogfood qualification sweep lost (want supported plus adapters plus env/docs/consumer/file/helper/clap/hello/parser/cli plus musl/macos/windows/matrix plus closeout)"
fi

# Per-host jobs: arm64 triple plus musl pairs plus macos arm64 triple plus windows triple.
if grep -q -F -e 'build-arm64 (bazel build' "$ci" &&
  grep -q -F -e 'test-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-arm64 (dx coverage gate, arm64 cell)' "$ci" &&
  grep -q -F -e 'build-musl-x86_64' "$ci" &&
  grep -q -F -e 'build-musl-arm64' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  grep -q -F -e 'build-macos-arm64' "$ci" &&
  grep -q -F -e 'test-macos-arm64' "$ci" &&
  grep -q -F -e 'coverage-macos-arm64' "$ci" &&
  ! grep -q -F -e 'build-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'test-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -q -F -e 'build-windows-x86_64' "$ci" &&
  grep -q -F -e 'test-windows-x86_64' "$ci" &&
  grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host job (arm64 triple plus musl pairs plus macos arm64 triple plus windows triple; x86_64 removed per #976)"
fi

# Host matrix stays pinned by ci_matrix_qualification.
if grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e 'runs-on: ubuntu-24.04-arm' "$ci" &&
  grep -q -F -e 'runs-on: macos-14' "$ci" &&
  ! grep -q -F -e 'runs-on: macos-15-intel' "$ci" &&
  grep -q -F -e 'runs-on: windows-latest' "$ci"; then
  ok
else
  bad "host matrix lost its ci_matrix_qualification plus four-runner pin (issue #415 plus #976)"
fi

# Battery documents the tail jobs: devcontainer plus docs-ci plus dogfood.
if grep -q -F -e 'devcontainer-check' "$ci" &&
  grep -q -F -e 'docs-ci (self-call reusable docs workflow)' "$ci" &&
  grep -q -F -e 'dogfood (self-call reusable consumer workflow)' "$ci"; then
  ok
else
  bad "Battery lost its devcontainer/docs-ci/dogfood tail (ci.yml)"
fi

# Docs gate: docs-ci self-call over //docs/... with publish only on main.
if grep -q -F -e 'reusable-docs' "$ci" &&
  grep -q -F -e 'docs_scope:' "$ci" &&
  grep -q -F -e '//docs/...' "$ci" &&
  grep -q -F -e 'docs_dir:' "$ci" &&
  grep -q -F -e 'publish:' "$ci" &&
  grep -q -F -e 'refs/heads/main' "$ci"; then
  ok
else
  bad "docs-ci gate lost its reusable-docs plus scope plus publish-on-main wiring"
fi

# Reusable-docs contract: check-only lint over the caller scope, validated
# tree, staged publish, clean checkout, no rendered site until.
if grep -q -F -e 'lint --check' "$reusable" &&
  grep -q -F -e 'validated docs tree' "$reusable" &&
  grep -q -F -e 'is pure check-only' "$reusable" &&
  grep -q -F -e 'RUNNER_TEMP' "$reusable" &&
  grep -q -F -e 'git status --porcelain' "$reusable" &&
  grep -q -F -e 'issue #581' "$reusable"; then
  ok
else
  bad "reusable-docs lost its check-only plus validated-tree plus clean-checkout contract (issue #581)"
fi

# Dogfood gate: test-disabled self-call on the four host platforms
# (Phase 1 coverage superset; starter stays all-nine).
if grep -q -F -e 'platforms:' "$ci" &&
  grep -q -F -e 'linux_x86_64' "$ci" &&
  grep -q -F -e 'linux_arm64' "$ci" &&
  grep -q -F -e 'macos_arm64' "$ci" &&
  ! grep -q -F -e 'macos_x86_64' "$ci" &&
  grep -q -F -e 'windows_x86_64' "$ci" &&
  grep -q -F -e 'disabled_checks: "test"' "$ci"; then
  ok
else
  bad "dogfood gate lost its test-disabled four-platform self-call (issue #408 plus Phase 1 #607; x86_64 removed per #976)"
fi

# Devcontainer gate: parity test plus definition shape, boot stays open gap.
if grep -q -F -e 'devcontainer_parity_test' "$ci" &&
  grep -q -F -e 'postCreateCommand' "$ci"; then
  ok
else
  bad "devcontainer-check lost its parity plus definition-shape wiring"
fi

# Testing strategy points to the short support matrix plus issues.
if grep -q -F -e 'support-matrix' "$testing_readme" &&
  grep -q -F -e 'GitHub issues' "$testing_readme" &&
  ! grep -q -F -e 'verification-matrix' "$testing_readme"; then
  ok
else
  bad "testing strategy lost its support-matrix plus issues pointer (#988)"
fi

dx_test_summary "close-out battery qualification harness"
