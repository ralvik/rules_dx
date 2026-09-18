#!/usr/bin/env bash
# E2E scenario-registration guard (issue #54, E2E-case convention).
#
# Every `integration/<name>/` child workspace carrying MODULE.bazel is a
# Stage 4 CLI-contract scenario that must actually run: referenced by a
# driver (`tools/ci/e2e.sh` args, `tools/ci/e2e_format.sh`, or future
# focused drivers) or by `tools/ci/BUILD.bazel` (sh_test args plus `:e2e`
# suite membership). An orphan scenario directory would silently never
# execute under either the explicit suite or wildcard suites (drivers
# are `manual`), so this guard fails `bazel test //...` instead.
#
# The structural convention itself (child workspace layout, driver
# reuse, manual sh_test + suite membership, explicit-only invocation,
# corpus carve-out) lives in `integration/README.md` ("Adding a case").
#
# Run by `bazel test //tools/ci:e2e_cases_test`; no CI workflow change
# (the `test` job already runs `bazel test //...`).
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

for mod in integration/*/MODULE.bazel; do
  name="$(basename "$(dirname "$mod")")"
  if grep -rq -F -e "$name" tools/ci/e2e.sh tools/ci/e2e_format.sh tools/ci/BUILD.bazel; then
    ok
  else
    bad "integration/$name/ is orphaned: no reference in tools/ci/e2e.sh, tools/ci/e2e_format.sh, or tools/ci/BUILD.bazel (see integration/README.md 'Adding a case')"
  fi
done

echo "e2e cases guard: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
