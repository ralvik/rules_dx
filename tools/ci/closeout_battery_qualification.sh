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
# - wiring: roadmap plus verification-matrix delivered records, BUILD
#   target, dogfood-freshness step, Battery accepted record with full-tree
#   green owned by CI via this harness;
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

roadmap="docs/roadmap.md"
verify="docs/testing/verification-matrix.md"
testing_readme="docs/testing/README.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
reusable=".github/workflows/reusable-docs.yml"

# Roadmap owns the delivered record under.
if grep -q -F -e 'close-out battery + docs delivered (issue #467' "$roadmap"; then
  ok
else
  bad "roadmap lost its close-out battery delivered record under #467"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'closeout_battery_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #467' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:closeout_battery_qualification' "$verify"; then
  ok
else
  bad "verification-matrix lost its #467 close-out battery qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':closeout_battery_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :closeout_battery_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`closeout_battery_qualification` 24/24' "$verify"; then
  ok
else
  bad "verification-matrix Green lost closeout_battery_qualification 24/24"
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
  grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' "$ci" &&
  grep -q -F -e '`build`: `bazel build //...`' "$verify"; then
  ok
else
  bad "build battery lost (want bazel build //... plus dx_dev smoke in ci.yml and Battery)"
fi

# Test battery: full test with hermetic CLI-contract pins, no manual.
# Tuned bounded flaky retries plus per-test timeout cap.
if grep -q -F -e 'bazel test --noshow_progress --flaky_test_attempts=3 --test_timeout=300 //...' "$ci" &&
  grep -q -F -e '- `test`: `bazel test --flaky_test_attempts=3 --test_timeout=300 //...`' "$verify"; then
  ok
else
  bad "test battery lost (want bazel test //... in ci.yml and Battery)"
fi

# Coverage battery: seed gate plus report guards.
if grep -q -F -e 'coverage --min-coverage 97 //...' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_report_guards' "$ci" &&
  grep -q -F -e '- `coverage`:' "$verify"; then
  ok
else
  bad "coverage battery lost (want dx coverage gate plus report guards in ci.yml and Battery)"
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
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:shell_contract' "$ci" &&
  grep -q -F -e '- `prove`:' "$verify"; then
  ok
else
  bad "prove battery lost (want twelve prove harnesses in ci.yml and Battery)"
fi

# Dogfood core: freshness plus ownership audits.
if grep -q -F -e 'generate --check //...' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:corpus_audit' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:code_ownership' "$ci" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:non_dogfed_paths' "$ci" &&
  grep -q -F -e '- `dogfood-freshness`:' "$verify"; then
  ok
else
  bad "dogfood core lost (want generate --check plus corpus/code/non-dogfed in ci.yml and Battery)"
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

# Per-host jobs: arm64 triple plus musl pairs plus macos triples plus windows triple.
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
  grep -q -F -e 'build-macos-x86_64' "$ci" &&
  grep -q -F -e 'test-macos-x86_64' "$ci" &&
  grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -q -F -e 'build-windows-x86_64' "$ci" &&
  grep -q -F -e 'test-windows-x86_64' "$ci" &&
  grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host job (arm64 triple plus musl pairs plus macos triples plus windows triple)"
fi

# Host matrix stays pinned by ci_matrix_qualification.
if grep -q -F -e 'ci_matrix_qualification' "$verify" &&
  grep -q -F -e 'issue #415' "$verify" &&
  grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e 'runs-on: ubuntu-24.04-arm' "$ci" &&
  grep -q -F -e 'runs-on: macos-14' "$ci" &&
  grep -q -F -e 'runs-on: macos-15-intel' "$ci" &&
  grep -q -F -e 'runs-on: windows-latest' "$ci"; then
  ok
else
  bad "host matrix lost its ci_matrix_qualification plus five-runner pin (issue #415)"
fi

# Battery documents the tail jobs: devcontainer plus docs-ci plus dogfood.
if grep -q -F -e 'devcontainer-check' "$verify" &&
  grep -q -F -e 'docs-ci' "$verify" &&
  grep -q -F -e 'dogfood (test-disabled self-call' "$verify" &&
  grep -q -F -e 'devcontainer-check' "$ci" &&
  grep -q -F -e 'docs-ci (self-call reusable docs workflow)' "$ci" &&
  grep -q -F -e 'dogfood (self-call reusable consumer workflow)' "$ci"; then
  ok
else
  bad "Battery lost its devcontainer/docs-ci/dogfood tail (verify plus ci.yml)"
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

# Dogfood gate: test-disabled self-call on the five host platforms
# (Phase 1 coverage superset; starter stays all-nine).
if grep -q -F -e 'platforms:' "$ci" &&
  grep -q -F -e 'linux_x86_64' "$ci" &&
  grep -q -F -e 'linux_arm64' "$ci" &&
  grep -q -F -e 'macos_arm64' "$ci" &&
  grep -q -F -e 'macos_x86_64' "$ci" &&
  grep -q -F -e 'windows_x86_64' "$ci" &&
  grep -q -F -e 'disabled_checks: "test"' "$ci"; then
  ok
else
  bad "dogfood gate lost its test-disabled five-platform self-call (issue #408 plus Phase 1 #607)"
fi

# Devcontainer gate: parity test plus definition shape, boot stays open gap.
if grep -q -F -e 'devcontainer_parity_test' "$ci" &&
  grep -q -F -e 'postCreateCommand' "$ci"; then
  ok
else
  bad "devcontainer-check lost its parity plus definition-shape wiring"
fi

# Full-tree green ownership: CI owns build/test green via this harness;
# no full rebuild is re-claimed here (static pins only).
if grep -q -F -e 'Full `build`/`test` green is owned by CI on this tree via' "$verify" &&
  grep -q -F -e 'closeout_battery_qualification' "$verify" &&
  grep -q -F -e 'issue #467' "$verify"; then
  ok
else
  bad "Battery lost its full build/test green CI ownership via closeout_battery_qualification (#467)"
fi

# Testing README keeps the close-out battery tracker pointer.
if grep -q -F -e 'close-out battery' "$testing_readme" &&
  grep -q -F -e 'verification-matrix' "$testing_readme"; then
  ok
else
  bad "testing README lost its close-out battery tracker pointer"
fi

# Remaining reds stay owned gaps, not green claims.
if grep -q -F -e 'Remaining reds stay owned gaps' "$verify"; then
  ok
else
  bad "verification-matrix lost its Remaining reds owned-gaps record"
fi

# Consumer aggregate stays stable: disabled-as-skipped green, else fail.
if grep -q -F -e 'consumer aggregate' "$verify" &&
  grep -q -F -e 'disabled-as-skipped' "$verify"; then
  ok
else
  bad "verification-matrix lost its consumer aggregate stable-identity record"
fi

# No Supported claim for the battery: Platform-qualified only, release open.
if grep -q -F -e 'No cell here is a `Supported` claim' "$verify" &&
  grep -q -F -e 'No cell is `Supported`' "$verify"; then
  ok
else
  bad "verification-matrix lost its no-Supported gate (battery never Supported)"
fi

dx_test_summary "close-out battery qualification harness"
