#!/usr/bin/env bash
# Close-out battery plus docs-gate qualification harness.
#
# Qualifies the as-built close-out battery plus docs gate with fixture
# evidence and owned gaps, without claiming Supported or full-tree green
# here:
# - delivered: battery commands pinned in `.github/workflows/ci.yml`
#   (dogfood four-platform consumer self-call plus the static-musl
#   build/coverage cells plus prove plus dogfood-freshness plus
#   devcontainer-check plus aggregate; raw per-host build/test/coverage
#   jobs are gone, each cell compiles once under dx) plus the docs gate
#   (reusable-docs check-only contract over `//docs/...`, called by the
#   pinned docs caller, not by ci.yml);
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
consumer=".github/workflows/reusable-consumer.yml"
docs_caller="examples/docs-ci/caller.yml"
bazelrc=".bazelrc"

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
if grep -q -F -e 'name = "closeout_battery_qualification"' tools/ci/ci_targets_c.bzl; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl lost the closeout_battery_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:closeout_battery_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "dogfood_freshness.sh lost the closeout_battery_qualification step"
fi

# Build battery: full build plus the adopt-rust dx_dev smoke.
if grep -q -F -e 'bazel build --noshow_progress //...' "$ci" &&
  grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' "$ci"; then
  ok
else
  bad "build battery lost (want bazel build //... plus dx_dev smoke in ci.yml)"
fi

# Test battery: raw per-host `bazel test //...` jobs are deliberately gone
# (each cell compiles once under dx); the four-platform dogfood self-call
# runs `dx test` on every host, `.bazelrc` owns the tuned flags for every
# entrypoint, and the one direct `bazel test` (devcontainer parity) keeps
# them inline.
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'dx -- test //...' "$consumer" &&
  grep -q -F -e 'test --flaky_test_attempts=3' "$bazelrc" &&
  grep -q -F -e 'test --test_timeout=300' "$bazelrc" &&
  grep -E -q 'bazel test --noshow_progress.*--flaky_test_attempts=3 --test_timeout=300' "$ci"; then
  ok
else
  bad "test battery lost (want the four-platform dogfood dx test self-call plus tuned direct bazel test in ci.yml)"
fi

# Coverage battery: seed gate plus report guards.
if grep -q -F -e 'coverage --min-coverage 97 //...' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_report_guards' "$ci"; then
  ok
else
  bad "coverage battery lost (want dx coverage gate plus report guards in ci.yml)"
fi

# Prove battery: the twelve prove harnesses stay wired (prove.sh).
prove="tools/ci/prove.sh"
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:target_tags' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_cell' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_spill' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:musl_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:macos_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:ci_matrix_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_hygiene' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_policy' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:publish_trust' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_contract' "$prove" &&
  grep -q -F -e 'tools/ci/prove.sh' "$ci"; then
  ok
else
  bad "prove battery lost (want twelve prove harnesses in prove.sh plus ci.yml step)"
fi

# Dogfood core: freshness plus ownership audits (dogfood_freshness.sh).
dogfood="tools/ci/dogfood_freshness.sh"
if grep -q -F -e 'generate --check //...' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:corpus_audit' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:code_ownership' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:non_dogfed_paths' "$dogfood" &&
  grep -q -F -e 'tools/ci:dogfood_freshness' "$ci"; then
  ok
else
  bad "dogfood core lost (want generate --check plus corpus/code/non-dogfed in dogfood_freshness.sh)"
fi

# Dogfood extended: qualification sweep stays wired (dogfood_freshness.sh).
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:supported_evidence_gate' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:quality_adapters_parity' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:env_codegen_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:docs_pipeline_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:consumer_ci_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:file_family_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:helper_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:clap_tokenizer_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:hello_smoke_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:parser_sample_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cli_contract_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:musl_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:macos_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:ci_matrix_qualification' "$dogfood" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:closeout_battery_qualification' "$dogfood" &&
  grep -q -F -e 'tools/ci:dogfood_freshness' "$ci"; then
  ok
else
  bad "dogfood qualification sweep lost (want supported plus adapters plus env/docs/consumer/file/helper/clap/hello/parser/cli plus musl/macos/windows/matrix plus closeout)"
fi

# Per-host jobs: the raw per-host triples are gone (each cell compiles
# once under dx); ci.yml keeps the static-musl build plus coverage pair
# while the macos arm64 plus windows hosts ride the consumer matrix in
# reusable-consumer.yml (macOS x86_64 removed per #976).
if grep -q -F -e 'build-musl-x86_64' "$ci" &&
  grep -q -F -e 'build-musl-arm64' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  grep -q -F -e "'macos-14'" "$consumer" &&
  grep -q -F -e "'windows-latest'" "$consumer" &&
  ! grep -q -F -e 'build-arm64' "$ci" &&
  ! grep -q -F -e 'test-arm64' "$ci" &&
  ! grep -q -F -e 'build-macos-arm64' "$ci" &&
  ! grep -q -F -e 'test-macos-arm64' "$ci" &&
  ! grep -q -F -e 'coverage-macos-arm64' "$ci" &&
  ! grep -q -F -e 'build-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'test-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'build-windows-x86_64' "$ci" &&
  ! grep -q -F -e 'test-windows-x86_64' "$ci" &&
  ! grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host job (want the musl build/coverage pair in ci.yml plus macos arm64 plus windows triples via reusable-consumer.yml; raw per-host jobs removed per #976)"
fi

# Host matrix stays pinned by ci_matrix_qualification: the Linux pair runs
# in ci.yml, macOS arm64 plus Windows x86_64 run only through the consumer
# matrix (freeness policy: no macos-latest, no macos-15-intel, no
# self-hosted, no larger).
if grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e 'runs-on: ubuntu-24.04-arm' "$ci" &&
  grep -q -F -e "'macos-14'" "$consumer" &&
  grep -q -F -e "'windows-latest'" "$consumer" &&
  ! grep -q -F -e 'runs-on: macos-14' "$ci" &&
  ! grep -q -F -e 'runs-on: macos-15-intel' "$ci" &&
  ! grep -q -F -e 'runs-on: windows-latest' "$ci" &&
  ! grep -E -q 'runs-on:.*(self-hosted|larger|macos-latest)' "$ci"; then
  ok
else
  bad "host matrix lost its four-runner pin (ubuntu pair in ci.yml plus macos-14 plus windows-latest via reusable-consumer.yml; issue #415 plus #976)"
fi

# Battery tail jobs: devcontainer plus dogfood self-call plus
# dogfood-freshness (the docs-ci job left ci.yml; its gate lives in
# reusable-docs.yml, called by the pinned docs caller).
if grep -q -F -e 'devcontainer-check' "$ci" &&
  grep -q -F -e 'dogfood-freshness' "$ci" &&
  grep -q -F -e 'dogfood (self-call reusable consumer workflow)' "$ci"; then
  ok
else
  bad "Battery lost its devcontainer/dogfood/dogfood-freshness tail (ci.yml)"
fi

# Docs gate: lives only in reusable-docs.yml now (no docs-ci job in
# ci.yml): scope plus dir plus publish inputs over //docs/... with
# lint --check, and publish wired to main by the pinned docs caller.
if grep -q -F -e 'docs_scope:' "$reusable" &&
  grep -q -F -e '//docs/...' "$reusable" &&
  grep -q -F -e 'docs_dir:' "$reusable" &&
  grep -q -F -e 'publish:' "$reusable" &&
  grep -q -F -e 'lint --check' "$reusable" &&
  grep -q -F -e 'refs/heads/main' "$docs_caller" &&
  grep -q -F -e 'reusable-docs.yml@' "$docs_caller" &&
  ! grep -q -F -e 'docs-ci (self-call reusable docs workflow)' "$ci"; then
  ok
else
  bad "docs-ci gate lost its reusable-docs plus scope plus publish-on-main wiring (gate lives only in reusable-docs.yml, issue #620)"
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

# Dogfood gate: all-nine self-call on the four host platforms (starter
# enables every check, so `dx test` runs per host; no disabled_checks
# input; macOS x86_64 removed per #976).
if grep -q -F -e 'platforms:' "$ci" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'min_coverage: "97"' "$ci" &&
  ! grep -q -F -e 'macos_x86_64' "$ci" &&
  ! grep -q -F -e 'disabled_checks' "$ci"; then
  ok
else
  bad "dogfood gate lost its all-nine four-platform self-call (issue #408 plus Phase 1 #607; x86_64 removed per #976)"
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
