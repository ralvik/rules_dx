#!/usr/bin/env bash
# Selective Maven `dx update` per-artifact wont-fix qualification (issue #634).
#
# Qualifies the Maven per-artifact decision with fixtures plus command docs:
# - decided: `maven:group:artifact` parses but stays WONT-FIX in V1.
#   `rules_jvm_external` offers no per-artifact pin target, exact pins in
#   `MODULE.bazel` stay constraints, and the whole-lock
#   `REPIN=1 bazel run @maven//:pin` refresh of
#   `third_party/jvm/maven_install.json` stays the only approved updater.
#   Selective fails closed as `unsupported` with the `dx update maven`
#   hint and never silently substitutes a full update;
# - fixtures: `cli/update/tests/fixtures/selective_maven/` (`pins.bzl`
#   plus `selective_maven.expected`) pins the disposition, approved
#   full argv, fail-closed hint, seed plus Jupiter identities, invalid
#   shapes, and rejected routes;
# - record: ADR 0024 owns the per-set rationale; the command docs carry
#   the Maven per-artifact note plus the fixture link;
# - scope: update-only, no lock format change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:selective_maven_qualification`,
# following //tools/ci:selective_update_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cli/update/tests/fixtures/selective_maven/pins.bzl"
expected="cli/update/tests/fixtures/selective_maven/selective_maven.expected"
fixture_build="cli/update/tests/fixtures/selective_maven/BUILD.bazel"
backend="cli/update/src/backend.rs"
selector="cli/update/src/selector.rs"
sets="cli/update/src/sets.rs"
exec_update="cli/cli/src/exec/update.rs"
exec_tests_a="cli/cli/src/exec/update_tests_a.rs"
module="MODULE.bazel"
command_doc="docs/cli/commands/audit-update-bazel.md"
adr="docs/decisions/0024-selective-update.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture pins stay present with the Maven wont-fix disposition under #634.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SELECTIVE_MAVEN = "wont-fix"' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #634' "$pins"; then
  ok
else
  bad "selective_maven pins fixture lost its wont-fix disposition under #634"
fi

# Pins record the approved full argv plus the fail-closed hint.
if grep -q -F -e 'REPIN=1 bazel run @maven//:pin' "$pins" &&
  grep -q -F -e 'use `dx update maven` for the set' "$pins" &&
  grep -q -F -e 'third_party/jvm/maven_install.json' "$pins" &&
  grep -q -F -e 'MODULE.bazel' "$pins"; then
  ok
else
  bad "selective_maven pins.bzl lost its full argv plus hint plus lock wiring under #634"
fi

# Pins record seed plus Jupiter identities plus invalid shapes plus rejected routes.
if grep -q -F -e 'maven:junit:junit' "$pins" &&
  grep -q -F -e 'maven:org.junit.jupiter:junit-jupiter-api' "$pins" &&
  grep -q -F -e 'maven:junit' "$pins" &&
  grep -q -F -e 'silent full-update substitution rejected' "$pins" &&
  grep -q -F -e 'private per-artifact emulation rejected' "$pins" &&
  grep -q -F -e 'hand-edited maven_install.json rejected' "$pins"; then
  ok
else
  bad "selective_maven pins.bzl lost its identities plus invalid plus rejected wiring under #634"
fi

# Expected fixture pins per-artifact wont-fix lines plus honesty lines.
if grep -q -F -e 'maven selective wont-fix' "$expected" &&
  grep -q -F -e 'maven:junit:junit parses but unsupported' "$expected" &&
  grep -q -F -e 'maven:org.junit.jupiter:junit-jupiter-api parses but unsupported' "$expected" &&
  grep -q -F -e 'silent full-update substitution rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #634' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "selective_maven.expected lost its per-artifact plus honesty lines under #634"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'selective_maven.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "selective_maven BUILD.bazel lost its pins plus expected exports with corpus under #634"
fi

# Backend keeps the Maven full pin (whole-lock refresh only).
if grep -q -F -e '(SetId::Maven, SetRequest::Full)' "$backend" &&
  grep -q -F -e 'bazel", "run", "@maven//:pin"' "$backend" &&
  grep -q -F -e 'REPIN' "$backend"; then
  ok
else
  bad "backend.rs lost its Maven whole-lock pin under #634"
fi

# Backend keeps the Maven selective Unsupported arm with the set hint.
if grep -q -F -e '(SetId::Maven, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e 'use `dx update maven` for the set' "$backend" &&
  grep -q -F -e 'BackendError::Unsupported' "$backend" &&
  grep -q -F -e 'maven_selective_reports_unsupported_with_set_hint' "$backend"; then
  ok
else
  bad "backend.rs lost its Maven selective Unsupported arm with hint under #634"
fi

# Selector parses group:artifact (seed plus Jupiter) and keeps full-wins.
if grep -q -F -e 'maven identities are group:artifact' "$selector" &&
  grep -q -F -e 'maven:org.junit.jupiter:junit-jupiter-api' "$selector" &&
  grep -q -F -e 'maven_selective_parses_group_artifact_and_stays_selective' "$selector" &&
  grep -q -F -e 'full wins' "$selector"; then
  ok
else
  bad "selector.rs lost its Maven group:artifact parse plus full-wins record under #634"
fi

# Live execution maps Unsupported to update_failed without launching.
if grep -q -F -e 'BackendError::Unsupported' "$exec_update" &&
  grep -q -F -e 'unsupported update:' "$exec_update" &&
  grep -q -F -e 'live_unsupported_selective_fails_without_launch' "$exec_tests_a"; then
  ok
else
  bad "exec/update.rs lost its unsupported-fails-without-launch execution under #634"
fi

# Set registry keeps the Maven manifest plus lock wiring.
if grep -q -F -e 'MODULE.bazel' "$sets" &&
  grep -q -F -e 'third_party/jvm/maven_install.json' "$sets" &&
  grep -q -F -e 'rules_jvm_external pin (REPIN=1 bazel run @maven//:pin)' "$sets"; then
  ok
else
  bad "sets.rs lost its Maven manifest plus lock wiring under #634"
fi

# MODULE pins the seed plus Jupiter artifacts with the fail-closed lock.
if grep -q -F -e 'junit:junit:4.13.2' "$module" &&
  grep -q -F -e 'org.junit.jupiter:junit-jupiter-api:6.1.3' "$module" &&
  grep -q -F -e 'lock_file = "//third_party/jvm:maven_install.json"' "$module" &&
  grep -q -F -e 'fail_if_repin_required = True' "$module"; then
  ok
else
  bad "MODULE.bazel lost its Maven seed plus Jupiter plus fail-closed lock wiring under #634"
fi

# ADR 0024 still owns the Maven wont-fix rationale.
if grep -q -F -e 'Maven `WONT-FIX`' "$adr" &&
  grep -q -F -e 'There is no per-artifact pin target' "$adr"; then
  ok
else
  bad "ADR 0024 lost its Maven wont-fix rationale underpinning #634"
fi

# Command docs carry the Maven per-set row plus the per-artifact note.
if grep -q -F -e '| maven | Wont-fix' "$command_doc" &&
  grep -q -F -e 'maven:org.junit.jupiter:junit-jupiter-api' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/selective_maven/' "$command_doc" &&
  grep -q -F -e 'issue #634' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its Maven row plus per-artifact note with fixture under #634"
fi

# ci_targets_c.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "selective_maven_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'selective_maven_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:selective_maven_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the selective_maven_qualification wiring (want target plus dogfood-freshness)"
fi

# Rejected routes stay rejected: no per-artifact argv, no hand-edited lock path.
if ! grep -q -F -e 'SetId::Maven, SetRequest::Packages(packages)' "$backend" &&
  grep -q -F -e 'hand-edited maven_install.json rejected' "$pins"; then
  ok
else
  bad "Maven selective gained a per-artifact executor or lost its hand-edit rejection under #634"
fi

dx_test_summary "selective maven qualification harness"
