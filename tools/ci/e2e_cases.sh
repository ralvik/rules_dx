#!/usr/bin/env bash
# E2E scenario-registration guard (issue #54, E2E-case convention).
#
# Every `integration/<name>/` child workspace carrying MODULE.bazel is a
# Stage 4 CLI-contract scenario that must actually run: referenced by a
# driver (`tools/ci/e2e.sh` args, `tools/ci/e2e_format.sh`,
# `tools/ci/e2e_preset.sh`, or future focused drivers) or by
# `tools/ci/BUILD.bazel` (sh_test args plus `:e2e` suite membership). An
# orphan scenario directory would silently never execute under either the
# explicit suite or wildcard suites (drivers are `manual`), so this guard
# fails `bazel test //...` instead.
#
# The structural convention itself (child workspace layout, driver
# reuse, manual sh_test + suite membership, explicit-only invocation,
# corpus carve-out) lives in `integration/README.md` ("Adding a case").
#
# Run by `bazel test //tools/ci:e2e_cases_test`; no CI workflow change
# (the `test` job already runs `bazel test //...`).
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

for mod in integration/*/MODULE.bazel; do
  name="$(basename "$(dirname "$mod")")"
  if grep -rq -F -e "$name" tools/ci/e2e.sh tools/ci/e2e_format.sh tools/ci/e2e_preset.sh tools/ci/BUILD.bazel; then
    ok
  else
    bad "integration/$name/ is orphaned: no reference in tools/ci/e2e.sh, tools/ci/e2e_format.sh, tools/ci/e2e_preset.sh, or tools/ci/BUILD.bazel (see integration/README.md 'Adding a case')"
  fi
done

dx_test_summary "e2e cases guard"
