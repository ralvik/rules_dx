#!/usr/bin/env bash
# Prove battery (split from `.github/workflows/ci.yml`). No behavior change.
set -euo pipefail

bazel run --noshow_progress //tools/ci:target_tags
bazel run --noshow_progress //tools/ci:coverage_cell
bazel run --noshow_progress //tools/ci:coverage_excludes_qualification
bazel run --noshow_progress //tools/ci:coverage_spill
bazel run --noshow_progress //tools/ci:coverage_qualification
bazel run --noshow_progress //tools/ci:macos_qualification
bazel run --noshow_progress //tools/ci:windows_qualification
bazel run --noshow_progress //tools/ci:ci_matrix_qualification
bazel run --noshow_progress //tools/ci:skip_budget_qualification
bazel run --noshow_progress //tools/ci:bootstrap_portability
bazel run --noshow_progress //tools/ci:flakiness_qualification
bazel run --noshow_progress //tools/ci:release_hygiene
bazel run --noshow_progress //tools/ci:release_policy
bazel run --noshow_progress //tools/ci:publish_trust
bazel run --noshow_progress //tools/ci:shell_contract
